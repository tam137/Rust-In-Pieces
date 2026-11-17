#!/bin/bash
#
# Logs, next to a training run, what the GPU driver and the memory manager do (see README, "Helper scripts").
#
# Every <interval> seconds one CSV line: the trainer's KFD queue eviction time (evicted_ms), GTT and VRAM
# use, a few /proc/meminfo fields and the compaction / page-migration counters of /proc/vmstat (running
# totals, not differences). When the kernel log reports a GPU reset, it writes <name>_reset_<n>.txt with the
# driver messages and the last five minutes of the CSV; the driver's devcoredump is copied if readable
# (usually root only). It only reads; it never changes the system. It ends when <pid> ends.
#
# Usage:
#   nnue_training/scripts/gpu_watch.sh <name> <pid> [interval] [out dir]
#
# Environment:
#   DRM_CARD   DRM card of the GPU under /sys/class/drm (default: card0)

set -u

if [ "$#" -lt 2 ]; then
    sed -n '3,15p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
fi

NAME="$1"; PARENT="$2"; INT="${3:-5}"; OUT="${4:-gpuwatch}"
DEV="/sys/class/drm/${DRM_CARD:-card0}/device"
mkdir -p "$OUT"; CSV="$OUT/$NAME.csv"
VM="compact_stall compact_fail compact_success compact_migrate_scanned compact_isolated compact_daemon_wake pgmigrate_success pgmigrate_fail pswpin pswpout thp_fault_alloc thp_fault_fallback drop_pagecache"
MI="MemFree MemAvailable Cached Dirty AnonPages Shmem"

[ -f "$CSV" ] || echo "time,epoch,trainer_pid,evicted_ms,gtt_used_mb,vram_used_mb,$(echo $MI | tr ' ' ','),$(echo $VM | tr ' ' ','),gpu_resets" > "$CSV"

mb() { [ -r "$1" ] && echo $(( $(cat "$1") / 1048576 )); }

resets=0; since=$(date +%s)
# Runs while <pid> exists and is not a zombie (exited, not yet reaped).
alive() { local st; st=$(awk '{sub(/.*\) /, ""); print $1}' "/proc/$1/stat" 2>/dev/null); [ -n "$st" ] && [ "$st" != Z ] && [ "$st" != X ]; }
while alive "$PARENT"; do
    now=$(date +%s)
    # The trainer is the KFD process whose parent is <pid>; otherwise take any KFD process.
    tp=""; ev=""
    for p in /sys/class/kfd/kfd/proc/*/; do
        [ -d "$p" ] || continue
        p="${p%/}"; pid="${p##*/}"
        [ "$(ps -o ppid= -p "$pid" 2>/dev/null | tr -d ' ')" = "$PARENT" ] && { tp="$pid"; break; }
        tp="$pid"
    done
    [ -n "$tp" ] && ev=$(cat /sys/class/kfd/kfd/proc/"$tp"/stats_*/evicted_ms 2>/dev/null | head -1)
    gtt=$(mb "$DEV/mem_info_gtt_used"); vram=$(mb "$DEV/mem_info_vram_used")
    mi=$(for k in $MI; do awk -v k="$k:" '$1==k {printf "%d", $2/1024}' /proc/meminfo; printf ","; done)
    vm=$(for k in $VM; do awk -v k="$k" '$1==k {printf "%s", $2}' /proc/vmstat; printf ","; done)
    n=$(journalctl -k -q --no-pager --since "@$since" 2>/dev/null | grep -c "GPU reset begin")
    since=$now
    if [ "$n" -gt 0 ]; then
        resets=$((resets + n)); f="$OUT/${NAME}_reset_$resets.txt"
        {
            echo "reset $resets at $(date +%F_%T), trainer pid $tp, evicted_ms $ev"
            echo "--- kernel log (last 3 min)"; journalctl -k -q --no-pager --since "-3min" 2>/dev/null | grep -i amdgpu | tail -60
            echo "--- CSV (last 5 min)"; head -1 "$CSV"; tail -$((300 / INT)) "$CSV"
        } > "$f"
        sleep 2
        copied=0
        for d in "$DEV/devcoredump/data" /sys/class/devcoredump/devcd*/data; do
            if [ -r "$d" ] && cp "$d" "$OUT/${NAME}_reset_${resets}.devcoredump" 2>/dev/null; then copied=1; break; fi
        done
        [ "$copied" = 1 ] && echo "devcoredump saved" >> "$f" || echo "devcoredump not readable (needs root)" >> "$f"
    fi
    echo "$(date +%T),$now,$tp,$ev,$gtt,$vram,${mi}${vm}$resets" >> "$CSV"
    sleep "$INT"
done
