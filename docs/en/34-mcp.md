# 34 · Built-in MCP server

> Condensed English translation. [Complete Chinese original](../../Doc/34-MCP-Server接入指南.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Overview

The kernel includes a Model Context Protocol server, REST credential/session management, tool-call audit, an AI access-center UI, and a `flamepanel mcp` stdio bridge for clients that do not use the HTTP transport directly.

Read-only tools, write tools with human approval, prompts/operations guidance, and the optional assistant have different capability boundaries. Do not grant an AI client unrestricted administrator credentials merely to connect it.

## Connection workflow

1. Open the panel's AI access center (`/ai-center`) and issue a scoped credential using the existing workflow.
2. Configure the client's supported transport and authentication method using the exact current settings in the full Chinese guide.
3. For stdio-only clients, use the existing `flamepanel mcp` bridge; keep secrets out of committed client configuration and shell history.
4. Verify allowed read operations and explicit denial/approval behavior using synthetic or authorized data.
5. Review sessions/tool-call audit and test revocation/expiry.

This condensed edition intentionally does not invent credential flags, client JSON, or a different MCP endpoint. The Chinese source retains Cursor/Claude Code/Desktop setup examples, protocol details, current tool catalog, prompts, and management endpoints.

## Safety and approval

Scopes and server permissions both matter. Tool execution must retain request validation, audit, and human-approval boundaries. A pending approval is not successful execution; cancellation, rejection, expiration, and replay need stable semantics. Never claim that a prose prompt is itself an authorization mechanism.

The assistant is optional and disabled by default. Provider credentials and response errors must be redacted. Updating documentation/comments must not alter protocol field names, tool names, or executable prompt/locale content.
