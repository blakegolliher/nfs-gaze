//go:build linux

package main

import (
	"testing"
	"time"
)

func TestParseEvents(t *testing.T) {
	tests := []struct {
		name     string
		parts    []string
		expected *NFSEvents
		wantErr  bool
	}{
		{
			name: "valid events with all fields",
			parts: []string{
				"1", "2", "3", "4", "5", "6", "7", "8", "9", "10",
				"11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
				"21", "22", "23", "24", "25", "26", "27",
			},
			expected: &NFSEvents{
				InodeRevalidate:  1,
				DentryRevalidate: 2,
				DataInvalidate:   3,
				AttrInvalidate:   4,
				VFSOpen:          5,
				VFSLookup:        6,
				VFSAccess:        7,
				VFSUpdatePage:    8,
				VFSReadPage:      9,
				VFSReadPages:     10,
				VFSWritePage:     11,
				VFSWritePages:    12,
				VFSGetdents:      13,
				VFSSetattr:       14,
				VFSFlush:         15,
				VFSFsync:         16,
				VFSLock:          17,
				VFSRelease:       18,
				CongestionWait:   19,
				SetattrTrunc:     20,
				ExtendWrite:      21,
				SillyRename:      22,
				ShortRead:        23,
				ShortWrite:       24,
				Delay:            25,
				PNFSRead:         26,
				PNFSWrite:        27,
			},
			wantErr: false,
		},
		{
			name:     "insufficient parts",
			parts:    []string{"1", "2", "3"},
			expected: &NFSEvents{},
			wantErr:  true,
		},
		{
			name: "minimum valid parts",
			parts: []string{
				"1", "2", "3", "4", "5", "6", "7", "8", "9", "10",
				"11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
				"21", "22", "23", "24", "25", "26", "27",
			},
			expected: &NFSEvents{
				InodeRevalidate:  1,
				DentryRevalidate: 2,
				DataInvalidate:   3,
				AttrInvalidate:   4,
				VFSOpen:          5,
				VFSLookup:        6,
				VFSAccess:        7,
				VFSUpdatePage:    8,
				VFSReadPage:      9,
				VFSReadPages:     10,
				VFSWritePage:     11,
				VFSWritePages:    12,
				VFSGetdents:     13,
				VFSSetattr:      14,
				VFSFlush:        15,
				VFSFsync:        16,
				VFSLock:         17,
				VFSRelease:      18,
				CongestionWait:  19,
				SetattrTrunc:    20,
				ExtendWrite:     21,
				SillyRename:     22,
				ShortRead:       23,
				ShortWrite:      24,
				Delay:           25,
				PNFSRead:        26,
				PNFSWrite:       27,
			},
			wantErr: false,
		},
		{
			name: "invalid number format",
			parts: []string{
				"invalid", "2", "3", "4", "5", "6", "7", "8", "9", "10",
				"11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
				"21", "22", "23", "24", "25", "26", "27",
			},
			expected: nil,
			wantErr:  true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := parseEvents(tt.parts)
			
			if tt.wantErr && err == nil {
				t.Errorf("parseEvents() expected error but got none")
				return
			}
			if !tt.wantErr && err != nil {
				t.Errorf("parseEvents() unexpected error: %v", err)
				return
			}
			
			if tt.expected == nil {
				if result != nil {
					t.Errorf("parseEvents() expected nil result but got: %v", result)
				}
				return
			}

			if result.InodeRevalidate != tt.expected.InodeRevalidate {
				t.Errorf("parseEvents() InodeRevalidate = %v, want %v", result.InodeRevalidate, tt.expected.InodeRevalidate)
			}
			if result.VFSOpen != tt.expected.VFSOpen {
				t.Errorf("parseEvents() VFSOpen = %v, want %v", result.VFSOpen, tt.expected.VFSOpen)
			}
			if result.PNFSRead != tt.expected.PNFSRead {
				t.Errorf("parseEvents() PNFSRead = %v, want %v", result.PNFSRead, tt.expected.PNFSRead)
			}
		})
	}
}

func TestCalculateDelta(t *testing.T) {
	tests := []struct {
		name     string
		oldOp    *NFSOperation
		newOp    *NFSOperation
		duration float64
		expected *DeltaStats
	}{
		{
			name: "valid delta calculation",
			oldOp: &NFSOperation{
				Name:        "READ",
				Ops:         100,
				BytesSent:   1000,
				BytesRecv:   5000,
				RTT:         500,
				ExecuteTime: 200,
				QueueTime:   100,
				Errors:      1,
				Timeouts:    2,
			},
			newOp: &NFSOperation{
				Name:        "READ",
				Ops:         150,
				BytesSent:   1500,
				BytesRecv:   7500,
				RTT:         750,
				ExecuteTime: 300,
				QueueTime:   150,
				Errors:      3,
				Timeouts:    4,
			},
			duration: 1.0,
			expected: &DeltaStats{
				Operation:    "READ",
				DeltaOps:     50,
				DeltaSent:    500,
				DeltaRecv:    2500,
				DeltaBytes:   3000,
				DeltaRTT:     250,
				DeltaExec:    100,
				DeltaQueue:   50,
				DeltaErrors:  2,
				DeltaRetrans: 2,
				IOPS:         50.0,
				AvgRTT:       5.0,
				AvgExec:      2.0,
				AvgQueue:     1.0,
				KBPerOp:      60.0 / 1024,
				KBPerSec:     3000.0 / 1024,
			},
		},
		{
			name:     "nil old operation",
			oldOp:    nil,
			newOp:    &NFSOperation{Name: "READ", Ops: 50},
			duration: 1.0,
			expected: nil,
		},
		{
			name:     "nil new operation",
			oldOp:    &NFSOperation{Name: "READ", Ops: 50},
			newOp:    nil,
			duration: 1.0,
			expected: nil,
		},
		{
			name: "no operation increase",
			oldOp: &NFSOperation{
				Name: "READ",
				Ops:  100,
			},
			newOp: &NFSOperation{
				Name: "READ",
				Ops:  100,
			},
			duration: 1.0,
			expected: &DeltaStats{
				Operation: "READ",
				DeltaOps:  0,
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := calculateDelta(tt.oldOp, tt.newOp, tt.duration)
			
			if tt.expected == nil {
				if result != nil {
					t.Errorf("calculateDelta() expected nil but got: %v", result)
				}
				return
			}
			
			if result == nil {
				t.Errorf("calculateDelta() expected result but got nil")
				return
			}

			if result.Operation != tt.expected.Operation {
				t.Errorf("calculateDelta() Operation = %v, want %v", result.Operation, tt.expected.Operation)
			}
			if result.DeltaOps != tt.expected.DeltaOps {
				t.Errorf("calculateDelta() DeltaOps = %v, want %v", result.DeltaOps, tt.expected.DeltaOps)
			}
			if result.IOPS != tt.expected.IOPS {
				t.Errorf("calculateDelta() IOPS = %v, want %v", result.IOPS, tt.expected.IOPS)
			}
		})
	}
}

func TestParseMountstats(t *testing.T) {
	// Test that the function handles non-existent files gracefully
	t.Run("non-existent file", func(t *testing.T) {
		result, err := parseMountstats("/nonexistent/file")
		if err == nil {
			t.Errorf("parseMountstats() expected error for non-existent file")
		}
		if result != nil {
			t.Errorf("parseMountstats() expected nil result for non-existent file")
		}
	})
}

func TestDisplayStatsNfsiostat(t *testing.T) {
	mount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
	}

	stats := []*DeltaStats{
		{
			Operation: "READ",
			DeltaOps:  50,
			IOPS:      25.0,
			KBPerSec:  100.0,
			KBPerOp:   2.0,
			AvgRTT:    5.0,
			AvgExec:   3.0,
			AvgQueue:  1.0,
		},
	}

	// Test that the function doesn't panic
	t.Run("no panic on display", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("displayStatsNfsiostat() panicked: %v", r)
			}
		}()
		
		// Capture output would require redirecting stdout
		// For now, just ensure it doesn't crash
		displayStatsNfsiostat(mount, stats, nil, false)
	})
}

func TestDisplayStatsSimple(t *testing.T) {
	mount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
	}

	stats := []*DeltaStats{
		{
			Operation: "READ",
			IOPS:      25.0,
			AvgRTT:    5.0,
			AvgExec:   3.0,
			KBPerSec:  100.0,
			KBPerOp:   2.0,
		},
	}

	// Test that the function doesn't panic
	t.Run("no panic on display", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("displayStatsSimple() panicked: %v", r)
			}
		}()
		
		// Capture output would require redirecting stdout
		// For now, just ensure it doesn't crash
		displayStatsSimple(mount, stats, false, time.Now())
	})
}