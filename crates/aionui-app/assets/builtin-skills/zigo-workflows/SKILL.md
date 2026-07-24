---
name: zigo-workflows
description: Create, inspect, update, activate, publish, execute, and troubleshoot the authenticated user's Zigo workflows through the zigo-workflows MCP. Use for every workflow lifecycle or execution-history request; never bypass the MCP through shell, containers, databases, credentials, runtime files, or internal service endpoints.
---

# Zigo Workflows

Use the mounted `zigo-workflows` MCP for the complete workflow lifecycle. Its tools are scoped to the authenticated current user.

## Tool map

| Goal | MCP tool |
| --- | --- |
| Find published workflows | `zigo_workflows_list` |
| Inspect a workflow definition | `zigo_workflow_get` |
| Create a draft | `zigo_workflow_create` |
| Change nodes, connections, settings, schedules, or metadata | `zigo_workflow_update` |
| Enable or disable automatic triggers | `zigo_workflow_activate` / `zigo_workflow_deactivate` |
| Put a saved version online or take it offline | `zigo_workflow_publish` / `zigo_workflow_unpublish` |
| Run the published snapshot | `zigo_workflow_execute` |
| Inspect run history | `zigo_executions_list` / `zigo_execution_get` |

There is no delete tool. Never simulate deletion through infrastructure access.

## Required operating method

1. Establish the desired outcome, trigger, inputs, outputs, integrations, error handling, and schedule. Ask only for details that materially affect the workflow.
2. Reuse before creating: list or inspect existing workflows when the request may refer to an existing workflow. If a name is ambiguous, present the matching names and IDs.
3. Build the smallest valid draft with explicit node names and connections. Preserve existing workflow fields that the user did not ask to change.
4. Read the workflow back after a create or update and check that nodes, connections, settings, and trigger semantics match the request.
5. Treat activation and publication as separate states:
   - activation controls automatic triggers;
   - publication exposes a saved snapshot for execution;
   - activating does not publish, and publishing does not imply activation.
6. Before activating, deactivating, publishing, unpublishing, or executing, obtain confirmation unless the user's current request explicitly asks for that exact action.
7. Execute only the published snapshot. For required form files, ask the user to attach them and pass their local paths in the MCP request. Do not invent file paths, credentials, IDs, or successful results.
8. For failures, inspect the execution list and the relevant execution record, identify the failing node and evidence, then propose or apply the smallest safe workflow update requested by the user.

## Security boundary

- Use only `zigo_workflow_*` and `zigo_execution_*` MCP tools for workflow discovery, creation, changes, scheduling, activation, publication, execution, and history.
- Never use shell commands or file-editing tools to access workflow containers, databases, internal HTTP endpoints, credentials, service configuration, or runtime files.
- Never attempt to enumerate or operate another user's workflows. Treat authorization and ownership errors as final security boundaries.
- Do not restart services after workflow updates; the supported API refreshes runtime state.
- If an MCP call fails, retry through the MCP only when the operation is safe and idempotent. Otherwise report the structured error and the next safe action.
- Never claim that a workflow was changed, published, activated, or executed without a successful MCP response.

## Response contract

After an operation, report:

- workflow name and ID;
- draft/published state;
- automatic-trigger active/inactive state when known;
- the action completed;
- execution ID and status for runs;
- any remaining user decision, required input, or validation warning.

Keep explanations concise, but show enough node-level detail for the user to verify the workflow.
