#!/usr/bin/env python3
"""Keeps the page cache of a training file small while a training run reads it.

Every <interval> seconds it calls posix_fadvise(DONTNEED) over the whole file, so the kernel drops the
file's cached pages. With the cache full, the memory manager kept compacting and migrating pages; every
migration evicted the trainer's GPU queue, and now and then the GPU firmware hung on that eviction (see
README, "Helper scripts"). Needs no root and does not touch the trainer; the trainer reads the file from
disk a little more often. Ends when the process <pid> ends.

Usage:
    nnue_training/scripts/cache_trim.py <file> <pid> <log> [interval seconds, default 20]
"""

import os
import sys
import time


def mem_free_mb():
    """MemFree from /proc/meminfo in MB, -1 if unavailable."""
    try:
        with open("/proc/meminfo") as f:
            for line in f:
                if line.startswith("MemFree:"):
                    return int(line.split()[1]) // 1024
    except OSError:
        pass
    return -1


def drop_cache(path):
    """Asks the kernel to drop the cached pages of `path`."""
    fd = os.open(path, os.O_RDONLY)
    try:
        os.posix_fadvise(fd, 0, 0, os.POSIX_FADV_DONTNEED)
    finally:
        os.close(fd)


def alive(pid):
    """True while `pid` runs; a zombie (exited, not yet reaped) counts as ended."""
    try:
        with open(f"/proc/{pid}/stat") as f:
            state = f.read().rsplit(")", 1)[1].split()[0]
    except (OSError, IndexError):
        return False
    return state not in ("Z", "X")


def run(path, pid, log, interval):
    with open(log, "a") as out:
        out.write(f"{time.strftime('%F %T')} start {os.path.basename(path)} every {interval}s\n")
        out.flush()
        while alive(pid):
            before = mem_free_mb()
            drop_cache(path)
            out.write(f"{time.strftime('%T')} MemFree {before} -> {mem_free_mb()} MB\n")
            out.flush()
            time.sleep(interval)
        out.write(f"{time.strftime('%F %T')} end\n")


def main(argv):
    if len(argv) < 4:
        print(__doc__, file=sys.stderr)
        return 2
    interval = float(argv[4]) if len(argv) > 4 else 20.0
    run(argv[1], int(argv[2]), argv[3], interval)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
