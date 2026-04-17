"""Fan-out helper. Runs `fn(item)` across items, optionally in parallel."""
from __future__ import annotations

import concurrent.futures
from collections.abc import Callable
from typing import TypeVar

T = TypeVar("T")
R = TypeVar("R")


def run_in_parallel(fn: Callable[[T], R], items: list[T],
                    parallelism: int = 1) -> list[R]:
    """Run fn over items. If parallelism > 1, uses a thread pool.

    Results come back in completion order (not input order).
    """
    if parallelism <= 1 or len(items) <= 1:
        return [fn(item) for item in items]
    with concurrent.futures.ThreadPoolExecutor(max_workers=parallelism) as pool:
        futures = [pool.submit(fn, item) for item in items]
        return [f.result() for f in concurrent.futures.as_completed(futures)]
