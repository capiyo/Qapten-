# AI Capability Discovery and Majority Routing

## User experience

A user describes the work without needing to know which provider or model is best. Axon classifies the capability required (for example, visual design, UI implementation, code review, research, or translation), then runs a small capability poll.

For a poll, Axon sends the same bounded task brief to a configured electorate of model adapters and asks each one to nominate the best eligible candidate for that specific capability. Each voter must provide a short rationale. Axon excludes self-nominations, accepts one ballot per voter, and counts votes deterministically.

The actual task is sent only after the poll closes and a strict majority exists. This separates cheap model discovery from expensive task execution.

## Voting rules

- The electorate and eligible candidate set are fixed when a poll is created.
- One ballot per voter; duplicate ballots are rejected.
- A voter cannot nominate itself.
- A nomination must target an eligible candidate.
- A strict majority is required: `floor(valid_ballots / 2) + 1`.
- A plurality, tie, or missing majority is not a winner. Axon must not silently route on a tie.
- The winning candidate is selected for the task; the nominations and rationales remain evidence in the routing record.
- The task itself is not broadcast to all ten models by default.

The Rust capability module implements the poll domain and strict-majority tally with unit tests. Polls and nominations are persisted in MongoDB with optimistic version checks, and the API exposes create, retrieve, nominate, and finalize operations. The Python `axon-route` worker supports configured OpenAI-compatible endpoints, asks eligible models to nominate in parallel, records valid ballots through the Axon API, and executes the task only on a strict-majority winner.

## API

- `POST /api/v1/workspaces/{workspace_id}/capability-polls`: create a poll with a fixed electorate and candidate catalog. The request must set `external_sharing_approved=true`; every voter must be in the eligible candidate catalog.
- `GET /api/v1/workspaces/{workspace_id}/capability-polls/{poll_id}`: retrieve the poll and current tally inputs.
- `POST /api/v1/workspaces/{workspace_id}/capability-polls/{poll_id}/nominations`: submit one voter nomination.
- `POST /api/v1/workspaces/{workspace_id}/capability-polls/{poll_id}/finalize`: close the poll and compute the strict-majority result.

Nomination and finalization writes use version-checked atomic document replacement. Concurrent submissions based on a stale version are rejected and must be retried after reloading.
\n## Running the router

Configure up to 30 models in `connectors/python/models.example.json` (copy it to a private local catalog and replace endpoint, model ID, and environment-variable names). Provider API keys stay in environment variables; they are never stored in the catalog. Models must declare the requested capability, and only models with configured keys participate.

Preview without sending the task:

```sh
axon-route --task "Design a responsive dashboard" --capability visual_design --workspace-id WORKSPACE_UUID --models ./models.json
```

After reviewing the provider list and task brief, explicitly approve sharing:

```sh
axon-route --task "Design a responsive dashboard" --capability visual_design --workspace-id WORKSPACE_UUID --models ./models.json --approve-provider-sharing
```

The worker asks the configured electorate in parallel. If fewer than three valid ballots return, it stops. If the poll has no strict majority, it does not dispatch the task. If a winner exists, only that model receives the execution request. The current worker returns the task result locally; it does not yet persist the result as a task response artifact.

## Why this is not an output-quality benchmark

This first-stage vote asks models to recommend a capable candidate. It is a discovery signal, not proof that the nominated model will produce the best output. Model self-promotion, correlated biases, stale knowledge, and incomplete provider knowledge can affect nominations.

The next stage should combine this vote with measured evidence:
- curated task-specific benchmark results;
- observed user acceptance and retry rates;
- latency, availability, and total cost;
- modality and tool compatibility;
- data residency and provider retention constraints.

A later version can optionally run two or three anonymous candidate outputs for subjective tasks such as design. That is a separate, opt-in quality comparison and should not multiply provider cost for every request.

## Privacy and cost

The default poll should contain a minimized task brief, not the user's entire conversation or private memory archive. Sensitive content must be screened and governed by workspace policy before it is sent to any external provider. Users must be able to restrict eligible providers.

A poll is a provider request itself. Axon should cache capability rankings and refresh them on model/provider changes or explicit user request rather than polling ten models for every routine task.

## Routing outcome

A completed decision should record:
- capability category and task brief digest;
- electorate and candidate catalog versions;
- each voter, nomination, and rationale;
- tally, quorum, and majority threshold;
- selected model and provider;
- estimated and actual cost, latency, and outcome feedback;
- policy version and timestamp.

If no majority is reached, Axon should ask for another round, use a previously validated benchmark ranking, or ask the user. It must not invent a winner.
