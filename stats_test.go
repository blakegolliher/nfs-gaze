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
				"abc", "2", "3", "4", "5", "6", "7", "8", "9", "10",
				"11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
				"21", "22", "23", "24", "25", "26", "27",
			},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := parseEvents(tt.parts)
			if (err != nil) != tt.wantErr {
				t.Errorf("parseEvents() error = %v, wantErr %v", err, tt.wantErr)
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
				if op.BytesSent != 1024 {
					t.Errorf("BytesSent = %d, want 1024", op.BytesSent)
				}
				if op.BytesRecv != 2048 {
					t.Errorf("BytesRecv = %d, want 2048", op.BytesRecv)
				}
				if op.QueueTime != 10 {
					t.Errorf("QueueTime = %d, want 10", op.QueueTime)
				}
				if op.RTT != 20 {
					t.Errorf("RTT = %d, want 20", op.RTT)
				}
				if op.ExecuteTime != 30 {
					t.Errorf("ExecuteTime = %d, want 30", op.ExecuteTime)
				}
				if op.Errors != 2 {
					t.Errorf("Errors = %d, want 2", op.Errors)
				}
			},
		},
		{
			name:    "valid operation without errors field",
			opName:  "WRITE",
			stats:   []string{"50", "50", "0", "512", "256", "5", "10", "15"},
			wantErr: true, // This should error because we require at least 9 fields
		},
		{
			name:    "insufficient stats",
			opName:  "GETATTR",
			stats:   []string{"100", "95"},
			wantErr: true,
		},
		{
			name:    "invalid number format",
			opName:  "LOOKUP",
			stats:   []string{"abc", "95", "5", "1024", "2048", "10", "20", "30"},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := parseNFSOperation(tt.opName, tt.stats)
			if (err != nil) != tt.wantErr {
				t.Errorf("parseNFSOperation() error = %v, wantErr %v", err, tt.wantErr)
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
			name: "single NFS mount",
			input: `device server.example.com:/export mounted on /mnt/nfs with fstype nfs4 statvers=1.1
	opts:	rw,vers=4.2,rsize=1048576,wsize=1048576
	age:	3600
	caps:	caps=0xffff
	sec:	flavor=1,pseudoflavor=1
	events:	100 200 300 400 500 600 700 800 900 1000 1100 1200 1300 1400 1500 1600 1700 1800 1900 2000 2100 2200 2300 2400 2500 2600 2700
	bytes:	1048576 0 0 0 2097152 0 0 0
	RPC iostats version: 1.0  p/v: 100003/4 (nfs)
	xprt:	tcp 0 0 1 0 0 100 100 0 100 0 2 1000 2000
	per-op statistics
	NULL: 1 1 0 40 24 5 0 5 0
	READ: 100 100 0 10240 204800 500 1000 1500 0
	WRITE: 50 50 0 102400 5120 250 500 750 0
	GETATTR: 200 200 0 20480 40960 1000 2000 3000 0`,
			verify: func(t *testing.T, mounts map[string]*NFSMount) {
				if len(mounts) != 1 {
					t.Fatalf("got %d mounts, want 1", len(mounts))
				}
				mount, ok := mounts["/mnt/nfs"]
				if !ok {
					t.Fatal("mount /mnt/nfs not found")
				}
				if mount.Server != "server.example.com" {
					t.Errorf("Server = %s, want server.example.com", mount.Server)
				}
				if mount.Export != "/export" {
					t.Errorf("Export = %s, want /export", mount.Export)
				}
				if mount.Age != 3600 {
					t.Errorf("Age = %d, want 3600", mount.Age)
				}
				if mount.BytesRead != 1048576 {
					t.Errorf("BytesRead = %d, want 1048576", mount.BytesRead)
				}
				if mount.BytesWrite != 2097152 {
					t.Errorf("BytesWrite = %d, want 2097152", mount.BytesWrite)
				}
				if len(mount.Operations) != 4 {
					t.Errorf("got %d operations, want 4", len(mount.Operations))
				}
				if readOp, ok := mount.Operations["READ"]; ok {
					if readOp.Ops != 100 {
						t.Errorf("READ.Ops = %d, want 100", readOp.Ops)
					}
				} else {
					t.Error("READ operation not found")
				}
			},
		},
		{
			name: "multiple NFS mounts",
			input: `device server1:/export1 mounted on /mnt/nfs1 with fstype nfs4 statvers=1.1
	age:	1000
	events:	1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27
	bytes:	100 0 0 0 200 0 0 0
device server2:/export2 mounted on /mnt/nfs2 with fstype nfs4 statvers=1.1
	age:	2000
	events:	10 20 30 40 50 60 70 80 90 100 110 120 130 140 150 160 170 180 190 200 210 220 230 240 250 260 270
	bytes:	300 0 0 0 400 0 0 0`,
			verify: func(t *testing.T, mounts map[string]*NFSMount) {
				if len(mounts) != 2 {
					t.Fatalf("got %d mounts, want 2", len(mounts))
				}
				mount1, ok := mounts["/mnt/nfs1"]
				if !ok {
					t.Fatal("mount /mnt/nfs1 not found")
				}
				if mount1.Server != "server1" {
					t.Errorf("mount1.Server = %s, want server1", mount1.Server)
				}
				mount2, ok := mounts["/mnt/nfs2"]
				if !ok {
					t.Fatal("mount /mnt/nfs2 not found")
				}
				if mount2.Server != "server2" {
					t.Errorf("mount2.Server = %s, want server2", mount2.Server)
				}
			},
		},
		{
			name: "NFS mount without colon in server",
			input: `device server mounted on /mnt/nfs with fstype nfs4 statvers=1.1
	age:	1000
	events:	1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27
	bytes:	100 0 0 0 200 0 0 0`,
			verify: func(t *testing.T, mounts map[string]*NFSMount) {
				mount := mounts["/mnt/nfs"]
				if mount.Server != "server" {
					t.Errorf("Server = %s, want server", mount.Server)
				}
				if mount.Export != "/" {
					t.Errorf("Export = %s, want /", mount.Export)
				}
			},
		},
		{
			name:  "empty input",
			input: "",
			verify: func(t *testing.T, mounts map[string]*NFSMount) {
				if len(mounts) != 0 {
					t.Errorf("got %d mounts, want 0", len(mounts))
				}
			},
		},
		{
			name: "non-NFS mounts should be ignored",
			input: `device /dev/sda1 mounted on / with fstype ext4
device server:/export mounted on /mnt/nfs with fstype nfs4 statvers=1.1
	age:	1000
	events:	1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27
	bytes:	100 0 0 0 200 0 0 0`,
			verify: func(t *testing.T, mounts map[string]*NFSMount) {
				if len(mounts) != 1 {
					t.Fatalf("got %d mounts, want 1", len(mounts))
				}
				if _, ok := mounts["/mnt/nfs"]; !ok {
					t.Fatal("NFS mount not found")
				}
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			reader := strings.NewReader(tt.input)
			got, err := parseMountstatsReader(reader)
			if (err != nil) != tt.wantErr {
				t.Errorf("parseMountstatsReader() error = %v, wantErr %v", err, tt.wantErr)
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
				BytesSent:   1000,
				BytesRecv:   2000,
				RTT:         500,
				ExecuteTime: 1000,
				QueueTime:   100,
				Errors:      2,
				Timeouts:    1,
			},
			currentOp: &NFSOperation{
				Name:        "READ",
				Ops:         200,
				BytesSent:   2000,
				BytesRecv:   4000,
				RTT:         1000,
				ExecuteTime: 2000,
				QueueTime:   200,
				Errors:      4,
				Timeouts:    2,
			},
			duration: 1.0,
			verify: func(t *testing.T, delta *DeltaStats) {
				if delta.DeltaOps != 100 {
					t.Errorf("DeltaOps = %d, want 100", delta.DeltaOps)
				}
				if delta.DeltaSent != 1000 {
					t.Errorf("DeltaSent = %d, want 1000", delta.DeltaSent)
				}
				if delta.DeltaRecv != 2000 {
					t.Errorf("DeltaRecv = %d, want 2000", delta.DeltaRecv)
				}
				if delta.DeltaBytes != 3000 {
					t.Errorf("DeltaBytes = %d, want 3000", delta.DeltaBytes)
				}
				if delta.IOPS != 100.0 {
					t.Errorf("IOPS = %f, want 100.0", delta.IOPS)
				}
				if delta.AvgRTT != 5.0 {
					t.Errorf("AvgRTT = %f, want 5.0", delta.AvgRTT)
				}
				if delta.AvgExec != 10.0 {
					t.Errorf("AvgExec = %f, want 10.0", delta.AvgExec)
				}
				if delta.KBPerOp != 3000.0/100.0/1024.0 {
					t.Errorf("KBPerOp = %f, want %f", delta.KBPerOp, 3000.0/100.0/1024.0)
				}
			},
		},
		{
			name: "no operations delta",
			previousOp: &NFSOperation{
				Name: "WRITE",
				Ops:  100,
			},
			currentOp: &NFSOperation{
				Name: "WRITE",
				Ops:  100,
			},
			duration: 1.0,
			verify: func(t *testing.T, delta *DeltaStats) {
				if delta.DeltaOps != 0 {
					t.Errorf("DeltaOps = %d, want 0", delta.DeltaOps)
				}
			},
		},
		{
			name:       "nil previous operation",
			previousOp: nil,
			currentOp: &NFSOperation{
				Name: "GETATTR",
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
			name: "multi-second duration",
			previousOp: &NFSOperation{
				Name:      "READ",
				Ops:       0,
				BytesSent: 0,
				BytesRecv: 0,
			},
			currentOp: &NFSOperation{
				Name:      "READ",
				Ops:       100,
				BytesSent: 10240,
				BytesRecv: 20480,
			},
			duration: 10.0,
			verify: func(t *testing.T, delta *DeltaStats) {
				if delta.IOPS != 10.0 {
					t.Errorf("IOPS = %f, want 10.0", delta.IOPS)
				}
				expectedKBPerSec := 30720.0 / 10.0 / 1024.0
				if delta.KBPerSec != expectedKBPerSec {
					t.Errorf("KBPerSec = %f, want %f", delta.KBPerSec, expectedKBPerSec)
				}
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := calculateDelta(tt.previousOp, tt.currentOp, tt.duration)
			if tt.verify != nil {
				tt.verify(t, got)
			}
		})
	}
}