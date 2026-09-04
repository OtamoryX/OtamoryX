# Event-Driven Background Workers

## Decision

SQLite remains the source of truth. An in-process `Notify` is only a wakeup hint:

1. Producers commit durable state first, then notify the relevant worker.
2. Workers claim work from SQLite after every wakeup.
3. Retryable work stores its next deadline and sleeps until that deadline.
4. Lease recovery remains a low-frequency safety net for crashed workers.

A lost notification is therefore harmless: startup recovery and lease recovery can restore
progress without turning the idle path into a write-heavy poller.

## Converted Workers

| Worker | Previous idle behavior | New wakeup | Durable fallback |
| --- | --- | --- | --- |
| Content-analysis reconciliation | Reclaimed a dependency job and wrote `pending` every 15 seconds | Dependency completion wakes matching archive jobs | AI queue lease reaper and startup queue drain |
| Content Profile | Claimed after a 5-second scan and ran a lease `UPDATE` on every pass | Archive ingestion or a behavior trigger notifies the profile worker | Ten-minute running-job recovery |
| Preference Learning | Scanned all behavior events every 10 seconds | A new behavior event and profile completion notify the learner | One startup enqueue scan and ten-minute running-job recovery |
| Preference Decision | Scanned completed analyses every 10 seconds | Analysis canonicalization and rule changes notify the decision worker | Retry deadline timer |
| AI retry scheduler | Already used a persisted `next_run_at` deadline | Queue notification or the exact retry timer | Thirty-second lease recovery for crashed attempts |

The old `spawn_content_analysis_worker` polling loop was removed. Content analysis is now owned by
the project AI queue worker.

## Intentionally Scheduled Work

Not every timer is an idle business-worker poll. The following remain scheduled because their
purpose is time-based rather than event-based:

- AI queue, Content Profile, and Preference Learning lease reapers recover work from a crashed
  process at a low frequency.
- AI, Content Profile, Preference Learning, and Preference Decision retry timers wake at a
  persisted retry deadline.
- Trash/session retention cleanup and in-memory login-rate-limit cleanup are housekeeping tasks.
- The filesystem watcher and provider backoff sleeps belong to their respective I/O or rate-limit
  mechanisms.

These paths do not scan for ordinary pending business work on their idle interval.

## Dependency State

`content_analysis_reconcile` and `auto_tagging` use `waiting_dependency` while waiting for OCR,
translation, metadata, or tagging inputs. OCR and auto-tagging can block the downstream workflow;
translation and metadata remain soft inputs for synthesis, but their outcomes still wake the
archive's reconciliation job so it can re-check the complete artifact set. Waiting jobs do not set
`next_run_at`, do not consume a retry attempt, and do not create another queue row. Batch
title-language detection extracts all archive IDs from its payload before waking them.

The active dedupe index includes `waiting_dependency`, so an explicit enqueue request upgrades the
existing durable row instead of creating a duplicate.

On process startup, persisted dependency waiters are promoted once for recovery; the next
reconciliation immediately re-checks the dependency state and waits again if its inputs are still
not terminal.

## SQLite Write Discipline

The main reduction is behavioral rather than a connection-pool trick. Idle workers no longer issue
claim `SELECT`s followed by lease `UPDATE`s, and waiting dependencies no longer rewrite queue rows
every few seconds. Writes now happen for an actual enqueue, claim, result, retry deadline, or
low-frequency recovery pass.

A single-writer connection pool could serialize writes, but it would add ownership and routing
complexity while leaving unnecessary writes intact. WAL mode, a bounded pool, short transactions,
and event-driven worker state are simpler and address the cause of the contention.

## Retry and Recovery Rules

- `Notify` is for immediate work and configuration changes; it is not durable state.
- `next_run_at`/`next_attempt_at` is for an intentional retry deadline.
- `waiting_dependency` and `waiting_analysis` have no periodic retry deadline.
- Lease reapers run at low frequency and only recover rows that are still marked `running` past
  their recovery threshold.
- Provider calls keep their existing thinking defaults, pacing, recovery chain, and lease budget.

Migration `0033_event_driven_background_workers` adds the retry index/column and expands active AI
queue deduplication to cover dependency-waiting rows.
