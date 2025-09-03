//go:build linux

package main

import "testing"

func TestNFSOperationCreation(t *testing.T) {
	op := &NFSOperation{
		Name:        "READ",
		Ops:         100,
		Ntrans:      95,
		Timeouts:    1,
		BytesSent:   1024,
		BytesRecv:   8192,
		QueueTime:   50,
		RTT:         25,
		ExecuteTime: 10,
		Errors:      2,
	}

	if op.Name != "READ" {
		t.Errorf("Expected Name to be 'READ', got %s", op.Name)
	}
	if op.Ops != 100 {
		t.Errorf("Expected Ops to be 100, got %d", op.Ops)
	}
	if op.BytesSent != 1024 {
		t.Errorf("Expected BytesSent to be 1024, got %d", op.BytesSent)
	}
}

func TestNFSEventsCreation(t *testing.T) {
	events := &NFSEvents{
		InodeRevalidate:  10,
		DentryRevalidate: 5,
		VFSOpen:          20,
		VFSLookup:        15,
	}

	if events.InodeRevalidate != 10 {
		t.Errorf("Expected InodeRevalidate to be 10, got %d", events.InodeRevalidate)
	}
	if events.VFSOpen != 20 {
		t.Errorf("Expected VFSOpen to be 20, got %d", events.VFSOpen)
	}
}

func TestNFSMountCreation(t *testing.T) {
	mount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
		Server:     "server",
		Export:     "/export",
		Age:        3600,
		Operations: make(map[string]*NFSOperation),
		Events:     &NFSEvents{},
		BytesRead:  1048576,
		BytesWrite: 524288,
	}

	if mount.Device != "server:/export" {
		t.Errorf("Expected Device to be 'server:/export', got %s", mount.Device)
	}
	if mount.MountPoint != "/mnt/nfs" {
		t.Errorf("Expected MountPoint to be '/mnt/nfs', got %s", mount.MountPoint)
	}
	if mount.Operations == nil {
		t.Error("Expected Operations map to be initialized")
	}
}

func TestDeltaStatsCreation(t *testing.T) {
	delta := &DeltaStats{
		Operation:  "WRITE",
		DeltaOps:   50,
		DeltaBytes: 2048,
		AvgRTT:     12.5,
		AvgExec:    8.2,
		IOPS:       25.0,
	}

	if delta.Operation != "WRITE" {
		t.Errorf("Expected Operation to be 'WRITE', got %s", delta.Operation)
	}
	if delta.IOPS != 25.0 {
		t.Errorf("Expected IOPS to be 25.0, got %f", delta.IOPS)
	}
}