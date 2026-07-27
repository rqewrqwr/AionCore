---
name: zigo-workflows
description: Create, inspect, update, activate, publish, execute, and troubleshoot the authenticated user's Zigo workflows, integration credentials, and data tables through the zigo-workflows MCP. Use for every workflow lifecycle, credential setup, data-table setup, or execution-history request; never bypass the MCP through shell, containers, databases, runtime files, or internal service endpoints.
---

# Zigo Workflows

Use the mounted `zigo-workflows` MCP for the complete workflow lifecycle. Its tools are scoped to the authenticated current user.

## Tool map

| Goal | MCP tool |
| --- | --- |
| List available credential metadata | `zigo_credentials_list` |
| Discover credential types and required fields | `zigo_credential_types_list` |
| Create or test an integration credential | `zigo_credential_create` / `zigo_credential_test` |
| List or create a Zigo data table | `zigo_data_tables_list` / `zigo_data_table_create` |
| Import, rename, or export a Zigo data table | `zigo_data_table_import_csv` / `zigo_data_table_rename` / `zigo_data_table_export_csv` |
| Inspect or change data-table columns | `zigo_data_table_columns_list` / `zigo_data_table_column_add` / `zigo_data_table_column_rename` / `zigo_data_table_column_move` |
| Read, insert, update, or upsert rows | `zigo_data_table_rows_list` / `zigo_data_table_rows_insert` / `zigo_data_table_row_update` / `zigo_data_table_rows_upsert` |
| Search nodes and inspect exact parameters | `zigo_node_types_search` / `zigo_node_type_get` |
| Build and validate a draft | `zigo_workflow_nodes_add` / `zigo_workflow_nodes_connect` / `zigo_workflow_node_parameters_update` / `zigo_workflow_validate` |
| Resolve dynamic node resources and fields | `zigo_node_parameter_options` / `zigo_node_resource_locator_results` / `zigo_node_resource_mapper_fields` |
| Find published workflows | `zigo_workflows_list` |
| Inspect a workflow definition | `zigo_workflow_get` |
| Create a draft | `zigo_workflow_create` |
| Change nodes, connections, settings, schedules, or metadata | `zigo_workflow_update` |
| Enable or disable automatic triggers | `zigo_workflow_activate` / `zigo_workflow_deactivate` |
| Put a saved version online or take it offline | `zigo_workflow_publish` / `zigo_workflow_unpublish` |
| Run the published snapshot | `zigo_workflow_execute` |
| Inspect, retry, or stop executions | `zigo_executions_list` / `zigo_execution_get` / `zigo_execution_retry` / `zigo_execution_stop` |

There is no delete tool. Never simulate deletion through infrastructure access.

## Required operating method

1. Establish the desired outcome, trigger, inputs, outputs, integrations, error handling, and schedule. Ask only for details that materially affect the workflow.
2. Reuse before creating: list or inspect existing workflows when the request may refer to an existing workflow. If a name is ambiguous, present the matching names and IDs.
3. Before constructing nodes, search for the relevant node types and inspect their exact parameters. Resolve dynamic options or resource fields through the mounted tools instead of guessing IDs or schemas.
4. Build the smallest valid draft with explicit node names and connections, run `zigo_workflow_validate`, and fix validation errors before saving. Preserve existing workflow fields that the user did not ask to change.
5. Read the workflow back after a create or update and check that nodes, connections, settings, and trigger semantics match the request.
6. Treat activation and publication as separate states:
   - activation controls automatic triggers;
   - publication exposes a saved snapshot for execution;
   - activating does not publish, and publishing does not imply activation.
7. Before activating, deactivating, publishing, unpublishing, executing, retrying, or stopping, obtain confirmation unless the user's current request explicitly asks for that exact action.
8. When a workflow requires credentials, list existing credential metadata and inspect the credential type schema first. Create or test credentials only when the user explicitly asks and supplies the real values. Never invent, repeat, expose, or read back secret values.
9. When a workflow requires structured storage, list existing Zigo data tables before creating a duplicate. Inspect the live column schema before writing. Create tables with explicit names and column types; use upsert only with stable match keys.
10. Execute only the published snapshot. For required form files or CSV imports, ask the user to attach the files and pass their real local paths in the MCP request. Do not invent file paths, credentials, IDs, or successful results.
11. For failures, inspect the execution list and the relevant execution record, identify the failing node and evidence, then propose or apply the smallest safe workflow update requested by the user. Retry or stop only when explicitly requested or confirmed.

## Security boundary

- Use only `zigo_credential_*`, `zigo_credentials_*`, `zigo_data_table_*`, `zigo_data_tables_*`, `zigo_node_*`, `zigo_workflow_*`, and `zigo_execution_*` MCP tools for credential setup, data tables, workflow discovery, creation, validation, changes, scheduling, activation, publication, execution, and history.
- Credential tools may list metadata and type schemas, create credentials, and test credentials. They cannot read stored secrets, modify credentials, or delete credentials.
- Never use shell commands or file-editing tools to access workflow containers, databases, internal HTTP endpoints, credentials, service configuration, or runtime files.
- Never attempt to enumerate or operate another user's workflows. Treat authorization and ownership errors as final security boundaries.
- Do not restart services after workflow updates; the supported API refreshes runtime state.
- If an MCP call fails, retry through the MCP only when the operation is safe and idempotent. Otherwise report the structured error and the next safe action.
- Never claim that a workflow was changed, published, activated, or executed without a successful MCP response.

## Product naming

- In all user-visible reasoning, plans, progress messages, and final answers, call the product and workflow engine **Zigo**.
- Do not mention the underlying implementation engine or its brand in user-visible text.
- Preserve internal API paths, node type identifiers, credential type identifiers, and tool arguments exactly as required; the naming rule changes presentation only and must never rewrite runtime identifiers.

## Response contract

After an operation, report:

- workflow name and ID;
- draft/published state;
- automatic-trigger active/inactive state when known;
- the action completed;
- execution ID and status for runs;
- any remaining user decision, required input, or validation warning.

Keep explanations concise, but show enough node-level detail for the user to verify the workflow.
