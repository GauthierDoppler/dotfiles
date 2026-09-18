---
name: notion
description: >-
  Read or write Notion from the terminal with `ntn`, the official Notion CLI. Use this WHENEVER
  the user wants to query a Notion database, read a page, create or edit a page, update page
  properties, or script an upsert against Notion — "pull the rows from my Notion DB", "update
  this row in Notion", "create a page under X", "what's in this Notion page", or any Notion URL
  or ID pasted with an instruction. Also use when `ntn` fails to authenticate.
---

# notion

`ntn` is installed by the dotfiles Brewfile (`cask "notion-cli"`). It is in beta: when a command
below fails, trust `ntn <cmd> --help` over this file.

## Auth

```bash
ntn doctor                 # what is missing
ntn login                  # browser flow, token lands in the keychain
```

`ntn pages` and `ntn datasources` need an integration token, not the login: set
`NOTION_API_TOKEN`. The integration must be connected to the page or database from Notion's
`...` menu, or every call is a 404.

## Read

```bash
ntn pages get <page-id-or-url>                       # Markdown, properties as frontmatter
ntn pages get <id> --json
ntn datasources query <db-id-or-url> --limit 50 --json
ntn datasources query <id> --filter '{"property":"Clé","rich_text":{"equals":"X"}}'
```

A database holds one or more data sources. A database ID or URL resolves to its single data
source; if that fails, `ntn datasources resolve <database-id>`.

## Write

```bash
ntn pages create --parent data-source:<id> --content '# Title\n\nBody'
ntn pages edit <id> < page.md                        # frontmatter from `get` is stripped
ntn api -X PATCH v1/pages/<id> properties[Status][select][name]=Done
ntn api -X PATCH v1/pages/<id> archived:=true
jq -n '{parent:{data_source_id:$d},properties:{...}}' --arg d <id> | ntn api v1/pages
```

`ntn pages` covers Markdown content only. Properties, templates and everything else go through
`ntn api`, which is a thin client over the public API: `path=value` for strings, `path:=json` for
typed values, `name==value` for query params, stdin or `-d @file.json` for a full body. Method is
GET, or POST when a body is present; `-X` overrides.

## Upsert a row

1. `ntn datasources query <id> --filter '{"property":"<key>","rich_text":{"equals":"<v>"}}' --json`
2. Empty `results` → `ntn api v1/pages` with `parent[data_source_id]=<id>`. Otherwise
   `ntn api -X PATCH v1/pages/<page-id>` with only the changed properties.

Never trash-and-recreate: page IDs are what other rows relate to.

## Discover

```bash
ntn api ls                          # every endpoint
ntn api v1/pages --help             # inputs for one endpoint
ntn api v1/pages --docs -X POST     # the official reference
```
