package main

import (
	"bytes"
	"io"
	"os"
	"strings"
	"testing"
	"time"
)

func captureOutput(f func()) string {
	old := os.Stdout
	r, w, _ := os.Pipe()
	os.Stdout = w

	f()

	w.Close()
	os.Stdout = old

	var buf bytes.Buffer
	io.Copy(&buf, r)
	return buf.String()
}

func TestDisplayStatsSimple(t *testing.T) {
	mount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
	}

	stats := []*DeltaStats{
		{
			Operation: "READ",
			DeltaOps:  100,
			IOPS:      100.0,
			AvgRTT:    1.5,
			AvgExec:   2.0,
			KBPerSec:  1024.0,
			KBPerOp:   10.24,
		},
		{
			Operation: "WRITE",
			DeltaOps:  50,
			IOPS:      50.0,
			AvgRTT:    2.5,
			AvgExec:   3.0,
			KBPerSec:  512.0,
			KBPerOp:   10.24,
		},
	}

	timestamp := time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC)

	t.Run("without bandwidth", func(t *testing.T) {
		output := captureOutput(func() {
			DisplayStatsSimple(mount, stats, false, timestamp)
		})

		if !strings.Contains(output, "READ") {
			t.Error("Output should contain READ operation")
		}
		if !strings.Contains(output, "WRITE") {
			t.Error("Output should contain WRITE operation")
		}
		if !strings.Contains(output, "100.0") {
			t.Error("Output should contain IOPS for READ")
		}
		if strings.Contains(output, "MB/s") {
			t.Error("Output should not contain bandwidth columns")
		}
	})

	t.Run("with bandwidth", func(t *testing.T) {
		output := captureOutput(func() {
			DisplayStatsSimple(mount, stats, true, timestamp)
		})

		if !strings.Contains(output, "READ") {
			t.Error("Output should contain READ operation")
		}
		if !strings.Contains(output, "WRITE") {
			t.Error("Output should contain WRITE operation")
		}
		if !strings.Contains(output, "MB/s") {
			t.Error("Output should contain bandwidth column")
		}
		if !strings.Contains(output, "KB/op") {
			t.Error("Output should contain KB/op column")
		}
	})

	t.Run("empty stats", func(t *testing.T) {
		output := captureOutput(func() {
			DisplayStatsSimple(mount, []*DeltaStats{}, false, timestamp)
		})

		if output != "" {
			t.Error("Output should be empty for empty stats")
		}
	})

	t.Run("nil stats", func(t *testing.T) {
		output := captureOutput(func() {
			DisplayStatsSimple(mount, nil, false, timestamp)
		})

		if output != "" {
			t.Error("Output should be empty for nil stats")
		}
	})

	t.Run("stats with zero delta ops", func(t *testing.T) {
		zeroStats := []*DeltaStats{
			{
				Operation: "READ",
				DeltaOps:  0,
			},
		}

		output := captureOutput(func() {
			DisplayStatsSimple(mount, zeroStats, false, timestamp)
		})

		if strings.Contains(output, "READ") {
			t.Error("Output should not contain operations with zero delta ops")
		}
	})
}