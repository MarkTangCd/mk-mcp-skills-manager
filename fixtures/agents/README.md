# Agent Configuration Fixtures

These directories contain **synthetic** sample configs for the four agents
AgentHub Local manages. They exist so adapters and ScanService can be
exercised without touching real user configuration.

## Rules

- Every file in this tree must use **fake** data only.
- No real API tokens, keys, OAuth secrets, machine paths, or identifying
  information may be committed.
- Paths inside fixtures should reference `/fixtures/...`-shaped absolute
  paths or relative paths under the fixture directory itself.
- Variables that look secret-ish must use placeholders like
  `${FAKE_TOKEN}` or `dummy-value`.

## Layout

```
fixtures/agents/
  <agent-kind>/
    empty/           # missing or empty config
    valid-global/    # well-formed global config
    valid-project/   # well-formed project-scoped config
    duplicate-mcp/   # two MCP entries that should collide
    invalid/         # malformed config (parser-level rejection)
```

Adapters select a variant by pointing `ScanContext.fixture_root` at the
fixture directory matching the variant under test.

## Variant semantics

| Variant         | What adapters should produce                                  |
| --------------- | ------------------------------------------------------------- |
| `empty`         | scan returns no resources, no errors                          |
| `valid-global`  | scan returns one or more resources tagged scope=global         |
| `valid-project` | scan returns resources tagged scope=project                   |
| `duplicate-mcp` | scan produces a warning / doctor issue for duplicate names     |
| `invalid`       | scan returns AdapterError::Parse (recoverable, not fatal)      |

Fixtures intentionally keep the smallest payload that lets a test
assert behavior — they are not meant to be exhaustive examples.
