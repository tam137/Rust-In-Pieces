#!/usr/bin/env python3
"""Tests for cache_trim.py: it drops a file's cached pages and ends with the watched process."""

import ctypes
import mmap
import os
import subprocess
import sys
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import cache_trim  # noqa: E402

PAGE = os.sysconf("SC_PAGE_SIZE")
SIZE = 64 * 1024 * 1024


def cached_pages(path):
    """Number of pages of `path` in the page cache, via mincore() on a read-only shared mapping."""
    libc = ctypes.CDLL(None, use_errno=True)
    libc.mmap.restype = ctypes.c_void_p
    libc.mmap.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_long]
    libc.munmap.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    libc.mincore.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p]
    size = os.path.getsize(path)
    pages = (size + PAGE - 1) // PAGE
    fd = os.open(path, os.O_RDONLY)
    try:
        address = libc.mmap(None, size, mmap.PROT_READ, mmap.MAP_SHARED, fd, 0)
        if address in (None, ctypes.c_void_p(-1).value):
            raise OSError(ctypes.get_errno(), "mmap failed")
        try:
            vec = (ctypes.c_ubyte * pages)()
            if libc.mincore(address, size, vec) != 0:
                raise OSError(ctypes.get_errno(), "mincore failed")
            return sum(v & 1 for v in vec)
        finally:
            libc.munmap(address, size)
    finally:
        os.close(fd)


class CacheTrimTest(unittest.TestCase):
    def setUp(self):
        # On disk next to this file: a tmpfs /tmp keeps its pages in memory and cannot drop them.
        self.dir = tempfile.TemporaryDirectory(dir=HERE, prefix=".test_cache_trim_")
        self.path = os.path.join(self.dir.name, "set.data")
        with open(self.path, "wb") as f:
            f.write(os.urandom(SIZE))
            f.flush()
            os.fsync(f.fileno())
        with open(self.path, "rb") as f:  # pull the whole file into the page cache
            while f.read(1 << 20):
                pass

    def tearDown(self):
        self.dir.cleanup()

    def test_drop_cache_evicts_the_files_pages(self):
        before = cached_pages(self.path)
        self.assertGreater(before, 0, "file not in the page cache to begin with")
        cache_trim.drop_cache(self.path)
        self.assertLess(cached_pages(self.path), before // 10)

    def test_the_script_ends_with_the_watched_process_and_logs(self):
        sleeper = subprocess.Popen(["sleep", "1"])
        log = os.path.join(self.dir.name, "trim.log")
        start = time.monotonic()
        subprocess.run([sys.executable, os.path.join(HERE, "cache_trim.py"), self.path, str(sleeper.pid), log, "0.2"],
                       check=True, timeout=20)
        sleeper.wait()
        self.assertLess(time.monotonic() - start, 10)
        with open(log) as f:
            lines = f.read().splitlines()
        self.assertIn("start set.data", lines[0])
        self.assertTrue(any("MemFree" in line for line in lines[1:-1]))
        self.assertTrue(lines[-1].endswith(" end"))

    def test_missing_arguments_print_the_usage(self):
        result = subprocess.run([sys.executable, os.path.join(HERE, "cache_trim.py")], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn("Usage", result.stderr)


if __name__ == "__main__":
    unittest.main()
