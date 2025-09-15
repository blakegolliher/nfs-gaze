package main

import (
	"strings"
	"testing"
)

func TestParseEvents(t *testing.T) {
	tests := []struct {
		name    string
		parts   []string
		wantErr bool
		verify  func(t *testing.T, e *NFSEvents)
	}{
		{
			name: "valid events with all fields",
			parts: []string{
				"1", "2", "3", "4", "5", "6", "7", "8", "9", "10",
				"11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
				"21", "22", "23", "24", "25", "26", "27",
			},
			wantErr: false,
			verify: func(t *testing.T, e *NFSEvents) {
				if e.InodeRevalidate != 1 {
					t.Errorf("InodeRevalidate = %d, want 1", e.InodeRevalidate)
				}
				if e.PNFSWrite != 27 {
					t.Errorf("PNFSWrite = %d, want 27", e.PNFSWrite)
				}
			},
		},
		{
			name: "valid events without pNFS fields",
			parts: []string{
				"100", "200", "300", "400", "500", "600", "700", "800", "900", "1000",
				"1100", "1200", "1300", "1400", "1500", "1600", "1700", "1800", "1900", "2000",
				"2100", "2200", "2300", "2400", "2500",
			},
			wantErr: true, // This should error because we require at least 27 parts
			verify: func(t *testing.T, e *NFSEvents) {
				// Not tested since it should error
			},
		},
		{
			name:    "insufficient parts",
			parts:   []string{"1", "2", "3"},
			wantErr: true,
		},
		{
			name: "invalid number format",
			parts: []string{
				"a", "2", "3", "4", "5", "6", "7", "8", "9", "10",
				"11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
				"21", "22", "23", "24", "25", "26", "27",
			},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := ParseEvents(tt.parts)
			if (err != nil) != tt.wantErr {
				t.Errorf("ParseEvents() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && tt.verify != nil {
				tt.verify(t, got)
			}
		})
	}
}

func TestParseNFSOperation(t *testing.T) {
	tests := []struct {
		name    string
		opName  string
		stats   []string
		wantErr bool
		verify  func(t *testing.T, op *NFSOperation)
	}{
		{
			name:   "valid operation with all fields",
			opName: "READ",
			stats:  []string{"100", "95", "5", "1024", "2048", "10", "20", "30", "2"},
			verify: func(t *testing.T, op *NFSOperation) {
				if op.Name != "READ" {
					t.Errorf("Name = %s, want READ", op.Name)
				}
				if op.Ops != 100 {
					t.Errorf("Ops = %d, want 100", op.Ops)
				}
				if op.Ntrans != 95 {
					t.Errorf("Ntrans = %d, want 95", op.Ntrans)
				}
				if op.Timeouts != 5 {
					t.Errorf("Timeouts = %d, want 5", op.Timeouts)
				}
			},
		},
		{
			name:   "valid operation without errors field",
			opName: "WRITE",
			stats:  []string{"50", "48", "2", "512", "1024", "5", "10", "15", "0"},
			verify: func(t *testing.T, op *NFSOperation) {
				if op.Name != "WRITE" {
					t.Errorf("Name = %s, want WRITE", op.Name)
				}
				if op.Ops != 50 {
					t.Errorf("Ops = %d, want 50", op.Ops)
				}
				if op.Errors != 0 {
					t.Errorf("Errors = %d, want 0", op.Errors)
				}
			},
		},
		{
			name:    "insufficient stats",
			opName:  "GETATTR",
			stats:   []string{"1", "2", "3"},
			wantErr: true,
		},
		{
			name:    "invalid number format",
			opName:  "LOOKUP",
			stats:   []string{"a", "2", "3", "4", "5", "6", "7", "8", "9"},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := ParseNFSOperation(tt.opName, tt.stats)
			if (err != nil) != tt.wantErr {
				t.Errorf("ParseNFSOperation() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && tt.verify != nil {
				tt.verify(t, got)
			}
		})
	}
}

func TestParseMountstatsReader(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		wantErr bool
		verify  func(t *testing.T, mounts map[string]*NFSMount)
	}{
		{
			name: "valid NFS mount",
			input: `device server1:/export on /mnt/nfs type nfs4 (rw,relatime)
	age:	3600
	events:	1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27
	bytes:	100 0 0 0 200 0 0 0
	READ: 10 10 0 1024 2048 50 100 150 0
	WRITE: 5 5 0 512 1024 25 50 75 0`,
			wantErr: false,
			verify: func(t *testing.T, mounts map[string]*NFSMount) {
				if len(mounts) != 1 {
					t.Errorf("Expected 1 mount, got %d", len(mounts))
				}
				mount, exists := mounts["/mnt/nfs"]
				if !exists {
					t.Error("Mount /mnt/nfs not found")
					return
				}
				if mount.Server != "server1" {
					t.Errorf("Server = %s, want server1", mount.Server)
				}
				if mount.Export != "/export" {
					t.Errorf("Export = %s, want /export", mount.Export)
				}
				if mount.Age != 3600 {
					t.Errorf("Age = %d, want 3600", mount.Age)
				}
				if len(mount.Operations) != 2 {
					t.Errorf("Expected 2 operations, got %d", len(mount.Operations))
				}
			},
		},
		{
			name: "multiple NFS mounts",
			input: `device server1:/export1 on /mnt/nfs1 type nfs4 (rw,relatime)
	age:	1800
	events:	1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27
	bytes:	50 0 0 0 100 0 0 0
device server2:/export2 on /mnt/nfs2 type nfs4 (ro,relatime)
	age:	900
	events:	1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27
	bytes:	25 0 0 0 50 0 0 0`,
			wantErr: false,
			verify: func(t *testing.T, mounts map[string]*NFSMount) {
				if len(mounts) != 2 {
					t.Errorf("Expected 2 mounts, got %d", len(mounts))
				}
			},
		},
		{
			name: "non-NFS mount (should be ignored)",
			input: `device /dev/sda1 on / type ext4 (rw,relatime)
device server:/export on /mnt/nfs type nfs4 (rw,relatime)
	age:	3600`,
			wantErr: false,
			verify: func(t *testing.T, mounts map[string]*NFSMount) {
				if len(mounts) != 1 {
					t.Errorf("Expected 1 NFS mount, got %d", len(mounts))
				}
			},
		},
		{
			name:    "empty input",
			input:   "",
			wantErr: false,
			verify: func(t *testing.T, mounts map[string]*NFSMount) {
				if len(mounts) != 0 {
					t.Errorf("Expected 0 mounts for empty input, got %d", len(mounts))
				}
			},
		},
		{
			name:    "malformed device line",
			input:   "device server:/export type nfs4",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			reader := strings.NewReader(tt.input)
			got, err := ParseMountstatsReader(reader)
			if (err != nil) != tt.wantErr {
				t.Errorf("ParseMountstatsReader() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && tt.verify != nil {
				tt.verify(t, got)
			}
		})
	}
}

func TestCalculateDelta(t *testing.T) {
	tests := []struct {
		name       string
		previousOp *NFSOperation
		currentOp  *NFSOperation
		duration   float64
		verify     func(t *testing.T, delta *DeltaStats)
	}{
		{
			name: "normal delta calculation",
			previousOp: &NFSOperation{
				Name:        "READ",
				Ops:         100,
				BytesSent:   1024,
				BytesRecv:   2048,
				RTT:         500,
				ExecuteTime: 1000,
				QueueTime:   100,
				Errors:      2,
				Timeouts:    1,
			},
			currentOp: &NFSOperation{
				Name:        "READ",
				Ops:         200,
				BytesSent:   2048,
				BytesRecv:   4096,
				RTT:         1000,
				ExecuteTime: 2000,
				QueueTime:   200,
				Errors:      4,
				Timeouts:    2,
			},
			duration: 1.0,
			verify: func(t *testing.T, delta *DeltaStats) {
				if delta.Operation != "READ" {
					t.Errorf("Operation = %s, want READ", delta.Operation)
				}
				if delta.DeltaOps != 100 {
					t.Errorf("DeltaOps = %d, want 100", delta.DeltaOps)
				}
				if delta.IOPS != 100.0 {
					t.Errorf("IOPS = %f, want 100.0", delta.IOPS)
				}
				if delta.AvgRTT != 5.0 {
					t.Errorf("AvgRTT = %f, want 5.0", delta.AvgRTT)
				}
			},
		},
		{
			name:       "nil previous operation",
			previousOp: nil,
			currentOp: &NFSOperation{
				Name: "WRITE",
				Ops:  50,
			},
			duration: 1.0,
			verify: func(t *testing.T, delta *DeltaStats) {
				if delta != nil {
					t.Error("Expected nil delta for nil previous operation")
				}
			},
		},
		{
			name: "nil current operation",
			previousOp: &NFSOperation{
				Name: "GETATTR",
				Ops:  50,
			},
			currentOp: nil,
			duration:  1.0,
			verify: func(t *testing.T, delta *DeltaStats) {
				if delta != nil {
					t.Error("Expected nil delta for nil current operation")
				}
			},
		},
		{
			name: "no operations increase",
			previousOp: &NFSOperation{
				Name: "LOOKUP",
				Ops:  100,
			},
			currentOp: &NFSOperation{
				Name: "LOOKUP",
				Ops:  100,
			},
			duration: 1.0,
			verify: func(t *testing.T, delta *DeltaStats) {
				if delta.DeltaOps != 0 {
					t.Errorf("DeltaOps = %d, want 0", delta.DeltaOps)
				}
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := CalculateDelta(tt.previousOp, tt.currentOp, tt.duration)
			if tt.verify != nil {
				tt.verify(t, got)
			}
		})
	}
}