import { spawn, spawnSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import type { Message } from "@earendil-works/pi-ai";
import { StringEnum } from "@earendil-works/pi-ai";
import { CONFIG_DIR_NAME, type ExtensionAPI, getAgentDir } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { type AgentConfig, type AgentScope, discoverAgents } from "./agents.ts";

type Runner = "headless" | "tmux" | "auto";
type Task = { agent: string; task: string; cwd?: string; model?: string; thinking?: string };

type Usage = {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  cost: number;
  turns: number;
};

type RunResult = {
  agent: string;
  task: string;
  source: "user" | "project" | "unknown";
  cwd: string;
  exitCode: number;
  output: string;
  stderr: string;
  usage: Usage;
  model?: string;
};

const MAX_PARALLEL = 8;
const MAX_CONCURRENCY = 4;

function getPiInvocation(args: string[]): { command: string; args: string[] } {
  const currentScript = process.argv[1];
  const isBunVirtualScript = currentScript?.startsWith("/$bunfs/root/");
  if (currentScript && !isBunVirtualScript && fs.existsSync(currentScript)) {
    return { command: process.execPath, args: [currentScript, ...args] };
  }
  const runtime = path.basename(process.execPath).toLowerCase();
  if (!/^(node|bun)(\.exe)?$/.test(runtime)) return { command: process.execPath, args };
  return { command: "pi", args };
}

function finalAssistantText(messages: Message[]): string {
  for (let i = messages.length - 1; i >= 0; i--) {
    const message = messages[i];
    if (message.role !== "assistant") continue;
    for (const part of message.content) {
      if (part.type === "text") return part.text;
    }
  }
  return "";
}

function formatUsage(usage: Usage): string {
  const parts: string[] = [];
  if (usage.turns) parts.push(`${usage.turns} turns`);
  if (usage.input) parts.push(`↑${usage.input}`);
  if (usage.output) parts.push(`↓${usage.output}`);
  if (usage.cacheRead) parts.push(`R${usage.cacheRead}`);
  if (usage.cacheWrite) parts.push(`W${usage.cacheWrite}`);
  if (usage.cost) parts.push(`$${usage.cost.toFixed(4)}`);
  return parts.join(" ");
}

function appendLog(logPath: string | undefined, text: string): void {
  if (!logPath) return;
  fs.appendFileSync(logPath, text, "utf8");
}

function truncate(value: string, max = 160): string {
  const normalized = value.replace(/\s+/g, " ").trim();
  return normalized.length > max ? `${normalized.slice(0, max - 1)}…` : normalized;
}

function formatToolArgs(toolName: string, args: Record<string, unknown>): string {
  if (toolName === "bash") return `$ ${truncate(String(args.command ?? ""), 220)}`;
  if (toolName === "read") return `read ${String(args.path ?? args.file_path ?? "")}`;
  if (toolName === "edit") return `edit ${String(args.path ?? args.file_path ?? "")}`;
  if (toolName === "write") return `write ${String(args.path ?? args.file_path ?? "")}`;
  if (toolName === "grep") return `grep ${String(args.pattern ?? "")} ${String(args.path ?? ".")}`;
  if (toolName === "find") return `find ${String(args.path ?? ".")} ${String(args.pattern ?? "*")}`;
  if (toolName === "ls") return `ls ${String(args.path ?? ".")}`;
  return `${toolName} ${truncate(JSON.stringify(args) ?? "")}`;
}

function formatToolResult(result: unknown, isError: boolean): string {
  const prefix = isError ? "✗" : "✓";
  const anyResult = result as any;
  const text = Array.isArray(anyResult?.content)
    ? anyResult.content.map((part: any) => (part?.type === "text" ? part.text : "")).join("\n")
    : typeof anyResult?.content === "string"
      ? anyResult.content
      : "";
  return text ? `${prefix} ${truncate(text, 220)}` : prefix;
}

function isInsideTmux(): boolean {
  return Boolean(process.env.TMUX) && spawnSync("tmux", ["display-message", "-p", "#S"], { stdio: "ignore" }).status === 0;
}

function findSubagentWindow(): string | undefined {
  const result = spawnSync("tmux", ["list-windows", "-F", "#{window_id}\t#{window_name}\t#{@pi_subagents}"], { encoding: "utf8" });
  if (result.status !== 0) return undefined;
  for (const line of result.stdout.split("\n")) {
    const [id, name, marker] = line.split("\t");
    if (id && marker === "1") return id;
    if (id && (name === "agents" || name?.startsWith("agents:"))) return id;
  }
  return undefined;
}

function markSubagentWindow(target: string): void {
  spawnSync("tmux", ["set-option", "-w", "-t", target, "automatic-rename", "off"], { stdio: "ignore" });
  spawnSync("tmux", ["set-option", "-w", "-t", target, "@pi_subagents", "1"], { stdio: "ignore" });
}

function createTmuxMonitorWindow(tasks: Task[]): { dir: string; logs: string[]; doneFiles: string[] } | undefined {
  if (!isInsideTmux()) return undefined;

  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "pi-subagents-"));
  const entries = tasks.map((task, index) => {
    const safeAgent = task.agent.replace(/[^a-zA-Z0-9_.-]+/g, "_");
    const base = `${String(index + 1).padStart(2, "0")}-${safeAgent}`;
    const logPath = path.join(dir, `${base}.log`);
    const donePath = path.join(dir, `${base}.done`);
    fs.writeFileSync(logPath, `subagent ${index + 1}/${tasks.length}: ${task.agent}\nTask: ${task.task}\n\n`, "utf8");
    return { logPath, donePath };
  });

  const tailScript = [
    "log=$1",
    "done=$2",
    "tail -n +1 -f \"$log\" &",
    "tail_pid=$!",
    "while [ ! -e \"$done\" ]; do sleep 0.2; done",
    "kill \"$tail_pid\" 2>/dev/null || true",
    "wait \"$tail_pid\" 2>/dev/null || true",
    "status=$(cat \"$done\" 2>/dev/null || echo 0)",
    "if [ \"$status\" = 0 ]; then printf '\\n\\033[32m✓ subagent done\\033[0m'; else printf '\\n\\033[31m✗ subagent failed — exit %s\\033[0m' \"$status\"; fi",
    "printf '  ·  any key to close'",
    "read -rsn1",
  ].join("\n");

  let target = findSubagentWindow();
  if (!target) {
    const created = spawnSync(
      "tmux",
      ["new-window", "-d", "-P", "-F", "#{window_id}", "-n", "agents", "bash", "-c", tailScript, "subagent-tail", entries[0].logPath, entries[0].donePath],
      { encoding: "utf8" },
    );
    target = created.status === 0 ? created.stdout.trim() : undefined;
    if (target) markSubagentWindow(target);
  } else {
    markSubagentWindow(target);
    spawnSync("tmux", ["split-window", "-d", "-t", target, "bash", "-c", tailScript, "subagent-tail", entries[0].logPath, entries[0].donePath], { stdio: "ignore" });
  }

  if (!target) return undefined;
  for (let i = 1; i < entries.length; i++) {
    spawnSync("tmux", ["split-window", "-d", "-t", target, "bash", "-c", tailScript, "subagent-tail", entries[i].logPath, entries[i].donePath], { stdio: "ignore" });
  }
  spawnSync("tmux", ["select-layout", "-t", target, "tiled"], { stdio: "ignore" });
  return { dir, logs: entries.map((entry) => entry.logPath), doneFiles: entries.map((entry) => entry.donePath) };
}

async function mapLimit<T, U>(items: T[], limit: number, fn: (item: T, index: number) => Promise<U>): Promise<U[]> {
  const results = new Array<U>(items.length);
  let next = 0;
  await Promise.all(
    Array.from({ length: Math.min(limit, items.length) }, async () => {
      while (true) {
        const index = next++;
        if (index >= items.length) return;
        results[index] = await fn(items[index], index);
      }
    }),
  );
  return results;
}

async function runAgent(options: {
  defaultCwd: string;
  dispatchModel?: string;
  dispatchThinking?: string;
  agents: AgentConfig[];
  task: Task;
  signal?: AbortSignal;
  logPath?: string;
  donePath?: string;
}): Promise<RunResult> {
  const { defaultCwd, dispatchModel, dispatchThinking, agents, task, signal, logPath, donePath } = options;
  const agent = agents.find((candidate) => candidate.name === task.agent);
  const cwd = task.cwd ?? defaultCwd;
  const usage: Usage = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, cost: 0, turns: 0 };

  if (!agent) {
    const output = `Unknown agent "${task.agent}". Available: ${agents.map((a) => a.name).join(", ") || "none"}`;
    appendLog(logPath, `${output}\n`);
    if (donePath) fs.writeFileSync(donePath, "1", "utf8");
    return { agent: task.agent, task: task.task, source: "unknown", cwd, exitCode: 1, output, stderr: "", usage };
  }

  const messages: Message[] = [];
  const args = ["--mode", "json", "-p", "--no-session"];
  const model = task.model ?? agent.model ?? dispatchModel;
  const thinking = task.thinking ?? agent.thinking ?? (!agent.model ? dispatchThinking : undefined);
  if (model) args.push("--model", model);
  if (thinking) args.push("--thinking", thinking);
  if (agent.tools?.length) args.push("--tools", agent.tools.join(","));
  if (agent.systemPrompt) args.push("--append-system-prompt", agent.systemPrompt);
  args.push(`Task delegated to subagent \"${agent.name}\":\n\n${task.task}`);

  const invocationPreview = getPiInvocation(args);
  appendLog(logPath, `cwd: ${cwd}\nrunner: ${invocationPreview.command} --mode json -p --no-session${model ? ` --model ${model}` : ""}${agent.tools?.length ? ` --tools ${agent.tools.join(",")}` : ""}\n\n`);

  let stderr = "";
  let aborted = false;
  const exitCode = await new Promise<number>((resolve) => {
    const invocation = getPiInvocation(args);
    const proc = spawn(invocation.command, invocation.args, { cwd, stdio: ["ignore", "pipe", "pipe"] });
    let buffer = "";

    let liveTextOpen = false;
    const processLine = (line: string) => {
      if (!line.trim()) return;
      let event: any;
      try {
        event = JSON.parse(line);
      } catch {
        return;
      }

      if (event.type === "turn_start") {
        appendLog(logPath, `\n━━ turn ${usage.turns + 1} ━━\n`);
        liveTextOpen = false;
        return;
      }

      if (event.type === "message_update") {
        const update = event.assistantMessageEvent;
        if (update?.type === "text_delta" && typeof update.delta === "string") {
          if (!liveTextOpen) {
            appendLog(logPath, "assistant: ");
            liveTextOpen = true;
          }
          appendLog(logPath, update.delta);
        }
        if (update?.type === "toolcall_start") {
          if (liveTextOpen) appendLog(logPath, "\n");
          liveTextOpen = false;
          appendLog(logPath, `tool: ${update.toolName ?? "unknown"}\n`);
        }
        return;
      }

      if (event.type === "tool_execution_start") {
        if (liveTextOpen) appendLog(logPath, "\n");
        liveTextOpen = false;
        appendLog(logPath, `→ ${formatToolArgs(event.toolName, event.args ?? {})}\n`);
        return;
      }

      if (event.type === "tool_execution_end") {
        appendLog(logPath, `${formatToolResult(event.result, Boolean(event.isError))}\n`);
        return;
      }

      if (event.type === "message_end" && event.message) {
        const message = event.message as Message;
        messages.push(message);
        if (message.role === "assistant") {
          if (liveTextOpen) appendLog(logPath, "\n");
          liveTextOpen = false;
          usage.turns += 1;
          const messageUsage = (message as any).usage;
          usage.input += messageUsage?.input ?? 0;
          usage.output += messageUsage?.output ?? 0;
          usage.cacheRead += messageUsage?.cacheRead ?? 0;
          usage.cacheWrite += messageUsage?.cacheWrite ?? 0;
          usage.cost += messageUsage?.cost?.total ?? 0;
        }
      }
    };

    proc.stdout.on("data", (chunk) => {
      buffer += chunk.toString();
      const lines = buffer.split("\n");
      buffer = lines.pop() ?? "";
      for (const line of lines) processLine(line);
    });
    proc.stderr.on("data", (chunk) => {
      const text = chunk.toString();
      stderr += text;
      appendLog(logPath, text);
    });
    proc.on("close", (code) => {
      if (buffer.trim()) processLine(buffer);
      resolve(code ?? 0);
    });
    proc.on("error", () => resolve(1));

    const kill = () => {
      aborted = true;
      proc.kill("SIGTERM");
      setTimeout(() => proc.kill("SIGKILL"), 5000).unref();
    };
    if (signal?.aborted) kill();
    else signal?.addEventListener("abort", kill, { once: true });
  });

  const finalExitCode = aborted ? 130 : exitCode;
  const output = aborted ? "Subagent aborted." : finalAssistantText(messages) || stderr || "(no output)";
  appendLog(logPath, `\n--- done: exit ${finalExitCode} ${formatUsage(usage)} ---\n`);
  if (donePath) fs.writeFileSync(donePath, String(finalExitCode), "utf8");
  return { agent: agent.name, task: task.task, source: agent.source, cwd, exitCode: finalExitCode, output, stderr, usage, model };
}

const TaskSchema = Type.Object({
  agent: Type.String({ description: "Agent name" }),
  task: Type.String({ description: "Task for this agent" }),
  cwd: Type.Optional(Type.String({ description: "Optional cwd override" })),
  model: Type.Optional(Type.String({ description: "Optional model override for this subagent, e.g. openai-codex/gpt-5.5 or anthropic/claude-sonnet-4-5" })),
  thinking: Type.Optional(Type.String({ description: "Optional thinking level override for this subagent: off, minimal, low, medium, high, xhigh, max" })),
});

const ParamsSchema = Type.Object({
  agent: Type.Optional(Type.String({ description: "Agent for single mode" })),
  task: Type.Optional(Type.String({ description: "Task for single mode" })),
  tasks: Type.Optional(Type.Array(TaskSchema, { description: "Parallel tasks. Max 8." })),
  chain: Type.Optional(Type.Array(TaskSchema, { description: "Sequential tasks. {previous} is replaced with previous output." })),
  cwd: Type.Optional(Type.String({ description: "Cwd for single mode" })),
  model: Type.Optional(Type.String({ description: "Optional model override for all launched subagents unless a task overrides it" })),
  thinking: Type.Optional(Type.String({ description: "Optional thinking level override for all launched subagents unless a task overrides it" })),
  runner: Type.Optional(StringEnum(["headless", "tmux", "auto"] as const, { description: "headless = no tmux; tmux = one monitor window; auto = tmux when available", default: "headless" })),
  agentScope: Type.Optional(StringEnum(["user", "project", "both"] as const, { description: "Load user/project agents", default: "user" })),
  background: Type.Optional(Type.Boolean({ description: "Start subagents and return immediately without waiting for results. Useful for fire-and-forget tasks.", default: false })),
  backgroundDelivery: Type.Optional(StringEnum(["discard", "followUp"] as const, { description: "For background runs: discard result, or send findings back as a follow-up user message when done.", default: "discard" })),
});

export default function (pi: ExtensionAPI) {
  pi.registerTool({
    name: "subagent",
    label: "Subagent",
    description: `Delegate to specialist Pi subagents with isolated context. Agents live in ${path.join(getAgentDir(), "agents")} and optionally ${CONFIG_DIR_NAME}/agents. Use runner=headless without tmux, runner=tmux for one tmux monitor window containing panes for all agents.`,
    parameters: ParamsSchema,
    async execute(_id, params, signal, onUpdate, ctx) {
      const scope = (params.agentScope ?? "user") as AgentScope;
      const runner = (params.runner ?? "headless") as Runner;
      const discovery = discoverAgents(ctx.cwd, scope);
      const hasSingle = Boolean(params.agent && params.task);
      const hasTasks = Boolean(params.tasks?.length);
      const hasChain = Boolean(params.chain?.length);
      const modes = Number(hasSingle) + Number(hasTasks) + Number(hasChain);
      if (modes !== 1) {
        return { content: [{ type: "text", text: `Provide exactly one mode. Available agents: ${discovery.agents.map((a) => `${a.name} (${a.source})`).join(", ") || "none"}` }], details: { results: [] }, isError: true };
      }

      const dispatchModel = params.model ?? (ctx.model ? `${ctx.model.provider}/${ctx.model.id}` : undefined);
      const dispatchThinking = params.thinking ?? ctx.thinkingLevel;
      const activeSignal = params.background ? undefined : signal;
      const activeOnUpdate = params.background ? undefined : onUpdate;
      const runTasks = async (tasks: Task[], chain: boolean) => {
        const wantsTmux = runner === "tmux" || (runner === "auto" && isInsideTmux());
        const monitor = wantsTmux ? createTmuxMonitorWindow(tasks) : undefined;
        if (wantsTmux && !monitor && runner === "tmux") ctx.ui.notify("Not inside tmux; running subagents headless.", "warning");

        if (chain) {
          const results: RunResult[] = [];
          let previous = "";
          for (let i = 0; i < tasks.length; i++) {
            const task = { ...tasks[i], task: tasks[i].task.replace(/\{previous\}/g, previous) };
            activeOnUpdate?.({ content: [{ type: "text", text: `Chain step ${i + 1}/${tasks.length}: ${task.agent}` }], details: { results } });
            const result = await runAgent({ defaultCwd: ctx.cwd, dispatchModel, dispatchThinking, agents: discovery.agents, task, signal: activeSignal, logPath: monitor?.logs[i], donePath: monitor?.doneFiles[i] });
            results.push(result);
            previous = result.output;
            if (result.exitCode !== 0) return results;
          }
          return results;
        }

        const results = await mapLimit(tasks, MAX_CONCURRENCY, async (task, index) => {
          activeOnUpdate?.({ content: [{ type: "text", text: `Running ${tasks.length} subagent(s)...` }], details: { running: true } });
          return runAgent({ defaultCwd: ctx.cwd, dispatchModel, dispatchThinking, agents: discovery.agents, task, signal: activeSignal, logPath: monitor?.logs[index], donePath: monitor?.doneFiles[index] });
        });
        return results;
      };

      let tasks: Task[];
      let mode: "single" | "parallel" | "chain";
      if (hasSingle) {
        mode = "single";
        tasks = [{ agent: params.agent!, task: params.task!, cwd: params.cwd, model: params.model, thinking: params.thinking }];
      } else if (hasTasks) {
        mode = "parallel";
        tasks = params.tasks!;
        if (tasks.length > MAX_PARALLEL) return { content: [{ type: "text", text: `Too many tasks: ${tasks.length}. Max ${MAX_PARALLEL}.` }], details: { results: [] }, isError: true };
      } else {
        mode = "chain";
        tasks = params.chain!;
      }

      const formatResults = (results: RunResult[]) =>
        results
          .map((result) => `## ${result.agent} (${result.exitCode === 0 ? "ok" : `failed ${result.exitCode}`})\n\n${result.output}\n\n_${formatUsage(result.usage)}${result.model ? ` · ${result.model}` : ""}_`)
          .join("\n\n---\n\n");

      if (params.background) {
        const delivery = params.backgroundDelivery ?? "discard";
        void runTasks(tasks, mode === "chain")
          .then((results) => {
            if (delivery !== "followUp") return;
            pi.sendUserMessage(
              `Background subagent result (${mode}, ${runner}):\n\n${formatResults(results)}\n\nPlease read these findings and decide the next step.`,
              { deliverAs: "followUp" },
            );
          })
          .catch((error) => {
            console.error("subagent background run failed", error);
            if (delivery === "followUp") {
              pi.sendUserMessage(`Background subagent failed: ${error instanceof Error ? error.message : String(error)}`, { deliverAs: "followUp" });
            }
          });
        return {
          content: [{ type: "text", text: `Started ${tasks.length} background subagent${tasks.length === 1 ? "" : "s"} (${mode}, ${runner}, delivery: ${delivery}).` }],
          details: { mode, runner, scope, background: true, backgroundDelivery: delivery, projectAgentsDir: discovery.projectAgentsDir, tasks },
        };
      }

      const results = await runTasks(tasks, mode === "chain");
      const failed = results.filter((result) => result.exitCode !== 0);
      const text = formatResults(results);
      return { content: [{ type: "text", text }], details: { mode, runner, scope, projectAgentsDir: discovery.projectAgentsDir, results }, isError: failed.length > 0 };
    },
  });

  pi.registerCommand("subagent-run", {
    description: "Run a subagent directly: /subagent-run [--tmux|--headless|--auto] [--scope user|project|both] <agent> <task>",
    handler: async (args, ctx) => {
      const tokens = args.trim().split(/\s+/).filter(Boolean);
      let runner: Runner = "headless";
      let scope: AgentScope = "user";
      let model: string | undefined;
      let thinking: string | undefined;
      let background = false;
      let backgroundDelivery: "discard" | "followUp" = "discard";
      while (tokens[0]?.startsWith("--")) {
        const flag = tokens.shift();
        if (flag === "--tmux") runner = "tmux";
        else if (flag === "--headless") runner = "headless";
        else if (flag === "--auto") runner = "auto";
        else if (flag === "--scope") scope = (tokens.shift() || "user") as AgentScope;
        else if (flag === "--model") model = tokens.shift();
        else if (flag === "--thinking") thinking = tokens.shift();
        else if (flag === "--background" || flag === "--bg") background = true;
        else if (flag === "--callback" || flag === "--follow-up") {
          background = true;
          backgroundDelivery = "followUp";
        }
      }

      const agent = tokens.shift();
      const taskText = tokens.join(" ");
      if (!agent || !taskText) {
        ctx.ui.notify("Usage: /subagent-run [--tmux|--headless|--auto] [--background|--bg|--callback] [--scope user|project|both] [--model provider/model] [--thinking level] <agent> <task>", "error");
        return;
      }

      const discovery = discoverAgents(ctx.cwd, scope);
      const task = { agent, task: taskText, model, thinking };
      const wantsTmux = runner === "tmux" || (runner === "auto" && isInsideTmux());
      const monitor = wantsTmux ? createTmuxMonitorWindow([task]) : undefined;
      if (wantsTmux && !monitor && runner === "tmux") ctx.ui.notify("Not inside tmux; running subagent headless.", "warning");

      const run = () => runAgent({
        defaultCwd: ctx.cwd,
        dispatchModel: model ?? (ctx.model ? `${ctx.model.provider}/${ctx.model.id}` : undefined),
        dispatchThinking: thinking ?? ctx.thinkingLevel,
        agents: discovery.agents,
        task,
        signal: background ? undefined : ctx.signal,
        logPath: monitor?.logs[0],
        donePath: monitor?.doneFiles[0],
      });

      if (background) {
        void run()
          .then((result) => {
            if (backgroundDelivery !== "followUp") return;
            pi.sendUserMessage(
              `Background subagent result (${result.agent}, ${runner}):\n\n${result.output}\n\nPlease read these findings and decide the next step.`,
              { deliverAs: "followUp" },
            );
          })
          .catch((error) => {
            console.error("subagent background command failed", error);
            if (backgroundDelivery === "followUp") {
              pi.sendUserMessage(`Background subagent failed: ${error instanceof Error ? error.message : String(error)}`, { deliverAs: "followUp" });
            }
          });
        ctx.ui.notify(`Started background subagent ${agent} (${runner}, delivery: ${backgroundDelivery}).`, "info");
        return;
      }

      ctx.ui.notify(`Running subagent ${agent} (${runner})...`, "info");
      const result = await run();
      ctx.ui.setEditorText(`Subagent ${result.agent} ${result.exitCode === 0 ? "succeeded" : `failed (${result.exitCode})`}\n\n${result.output}`);
    },
  });

  pi.registerCommand("agents", {
    description: "List available subagents",
    handler: async (args, ctx) => {
      const scope = (args.trim() || "user") as AgentScope;
      const { agents, projectAgentsDir } = discoverAgents(ctx.cwd, scope);
      const lines = agents.map((agent) => `- ${agent.name} (${agent.source}) — ${agent.description}`);
      ctx.ui.notify(`Agents (${scope})\n${lines.join("\n") || "none"}${projectAgentsDir ? `\nproject: ${projectAgentsDir}` : ""}`, "info");
    },
  });
}
