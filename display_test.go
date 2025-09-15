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

func TestDisplayStatsNfsiostat(t *testing.T) {
	mount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
		Events: &NFSEvents{
			VFSOpen:         100,
			InodeRevalidate: 200,
			DataInvalidate:  50,
			AttrInvalidate:  25,
		},
	}

	stats := []*DeltaStats{
		{
			Operation:    "READ",
			DeltaOps:     100,
			DeltaSent:    1024,
			DeltaRecv:    2048,
			DeltaBytes:   3072,
			DeltaRTT:     500,
			DeltaExec:    1000,
			DeltaQueue:   100,
			DeltaErrors:  2,
			DeltaRetrans: 1,
			IOPS:         100.0,
			AvgRTT:       5.0,
			AvgExec:      10.0,
			AvgQueue:     1.0,
			KBPerOp:      3.0,
			KBPerSec:     300.0,
		},
		{
			Operation:    "WRITE",
			DeltaOps:     50,
			DeltaSent:    2048,
			DeltaRecv:    512,
			DeltaBytes:   2560,
			DeltaRTT:     250,
			DeltaExec:    500,
			DeltaQueue:   50,
			DeltaErrors:  0,
			DeltaRetrans: 0,
			IOPS:         50.0,
			AvgRTT:       5.0,
			AvgExec:      10.0,
			AvgQueue:     1.0,
			KBPerOp:      2.5,
			KBPerSec:     125.0,
		},
	}

	previousMount := &NFSMount{
		Events: &NFSEvents{
			VFSOpen:         90,
			InodeRevalidate: 180,
			DataInvalidate:  45,
			AttrInvalidate:  20,
		},
	}

	tests := []struct {
		name     string
		showAttr bool
		verify   func(t *testing.T, output string)
	}{
		{
			name:     "without attributes",
			showAttr: false,
			verify: func(t *testing.T, output string) {
				if !strings.Contains(output, "server:/export mounted on /mnt/nfs:") {
					t.Error("Missing mount header")
				}
				if !strings.Contains(output, "ops/s") {
					t.Error("Missing ops/s header")
				}
				if !strings.Contains(output, "read:") {
					t.Error("Missing READ operation")
				}
				if !strings.Contains(output, "write:") {
					t.Error("Missing WRITE operation")
				}
				if strings.Contains(output, "VFS opens") {
					t.Error("Should not show VFS opens when showAttr is false")
				}
			},
		},
		{
			name:     "with attributes",
			showAttr: true,
			verify: func(t *testing.T, output string) {
				if !strings.Contains(output, "10 VFS opens") {
					t.Error("Missing VFS opens count")
				}
				if !strings.Contains(output, "20 inoderevalidates") {
					t.Error("Missing inode revalidates count")
				}
				if !strings.Contains(output, "5 page cache invalidations") {
					t.Error("Missing page cache invalidations")
				}
				if !strings.Contains(output, "5 attribute cache invalidations") {
					t.Error("Missing attribute cache invalidations")
				}
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			output := captureOutput(func() {
				displayStatsNfsiostat(mount, stats, previousMount, tt.showAttr)
			})
			tt.verify(t, output)
		})
	}
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
			AvgRTT:    5.123,
			AvgExec:   10.456,
			KBPerSec:  1024.0,
			KBPerOp:   10.24,
		},
		{
			Operation: "WRITE",
			DeltaOps:  50,
			IOPS:      50.0,
			AvgRTT:    6.789,
			AvgExec:   12.345,
			KBPerSec:  512.0,
			KBPerOp:   10.24,
		},
	}

	timestamp := time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC)

	tests := []struct {
		name          string
		showBandwidth bool
		verify        func(t *testing.T, output string)
	}{
		{
			name:          "without bandwidth",
			showBandwidth: false,
			verify: func(t *testing.T, output string) {
				if !strings.Contains(output, "server:/export mounted on 01/01/2024 12:00:00 PM") {
					t.Error("Missing mount header with timestamp")
				}
				if !strings.Contains(output, "Operation") && !strings.Contains(output, "IOPS") {
					t.Error("Missing column headers")
				}
				if !strings.Contains(output, "READ") {
					t.Error("Missing READ operation")
				}
				if !strings.Contains(output, "WRITE") {
					t.Error("Missing WRITE operation")
				}
				if strings.Contains(output, "MB/s") || strings.Contains(output, "KB/op") {
					t.Error("Should not show bandwidth columns when showBandwidth is false")
				}
			},
		},
		{
			name:          "with bandwidth",
			showBandwidth: true,
			verify: func(t *testing.T, output string) {
				if !strings.Contains(output, "MB/s") {
					t.Error("Missing MB/s column")
				}
				if !strings.Contains(output, "KB/op") {
					t.Error("Missing KB/op column")
				}
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			output := captureOutput(func() {
				displayStatsSimple(mount, stats, tt.showBandwidth, timestamp)
			})
			tt.verify(t, output)
		})
	}
}

func TestDisplayStatsSimpleEmptyStats(t *testing.T) {
	mount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
	}

	// Test with empty stats
	output := captureOutput(func() {
		displayStatsSimple(mount, []*DeltaStats{}, false, time.Now())
	})

	if output != "" {
		t.Error("Expected no output for empty stats")
	}

	// Test with nil stats
	output = captureOutput(func() {
		displayStatsSimple(mount, nil, false, time.Now())
	})

	if output != "" {
		t.Error("Expected no output for nil stats")
	}
}

func TestDisplayStatsNfsiostatWithNilStats(t *testing.T) {
	mount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
		Events:     &NFSEvents{},
	}

	// Test with stats containing nil entries
	stats := []*DeltaStats{
		nil,
		{
			Operation: "READ",
			DeltaOps:  0, // Zero ops should be skipped
		},
		{
			Operation: "WRITE",
			DeltaOps:  10,
			IOPS:      10.0,
			AvgRTT:    5.0,
			AvgExec:   10.0,
		},
	}

	output := captureOutput(func() {
		displayStatsNfsiostat(mount, stats, nil, false)
	})

	if !strings.Contains(output, "write:") {
		t.Error("Should display WRITE operation")
	}
	if strings.Contains(output, "read:") {
		t.Error("Should not display READ operation with 0 ops")
	}
}