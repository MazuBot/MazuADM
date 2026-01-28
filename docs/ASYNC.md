# Async/Tokio usage overview

This document summarizes backend async usage (core + api) and the refactor/test goals.

## Core

### core/src/executor.rs
- Spawns one task per job via tokio::spawn with a semaphore to cap concurrency.
- Uses per-target locks (Arc<Mutex<()>>) to enforce sequential execution when enabled.
- Performs small jitter sleeps to spread container reuse.
- Broadcasts updates via tokio::sync::broadcast.

Risks/notes:
- Detached tasks were joined without error inspection.
- Random jitter makes job timing nondeterministic.

### core/src/container_manager.rs
- Uses tokio::select! over exec output streams and timeouts.
- Kills process on timeout/over-limit.

## API

### api/src/handlers.rs
- WebSocket handler uses tokio::select! between broadcast receiver and client socket.
- Round execution is handled by a single SchedulerRunner background task.
- Handlers enqueue scheduler commands and notify the runner.
- Single job execution spawns a detached async task in run_job_internal.
 - Scheduler.run_round stops all running jobs immediately before scheduling a new round.

Risks/notes:
- Detached tasks do not log errors on failure.
- Some handler flow control is not easily unit-testable without DB.

## Refactor goals
- Improve determinism in async scheduling.
- Make spawn points explicit and log failures.
- Extract pure helpers for unit tests (round selection/finalization and status logic).
