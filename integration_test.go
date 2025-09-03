//go:build linux

package main

import (
	"flag"
	"io/ioutil"
	"os"
	"testing"
	"time"
)

// Test parseMountstats with actual mountstats-like data
func TestParseMountstatsIntegration(t *testing.T) {
	// Create a temporary file with sample mountstats data
	mountstatsData := `device server1:/export1 mounted on /mnt/nfs1 with fstype nfs4 statvers=1.1
	opts:	rw,vers=4.1,rsize=1048576,wsize=1048576,namlen=255,hard,proto=tcp,timeo=600,retrans=2
	age:	3661
	impl_id:	name='',domain='',date='0,0'
	caps:	caps=0xbfff,wtmult=512,dtsize=32768,bsize=0,namlen=255
	nfsv4:	bm0=0xfdffbfff,bm1=0x40f9be3e,bm2=0x803,acl=0x3,sessions,pnfs=not configured
	sec:	flavor=1,pseudoflavor=1
	events:	27 126 0 0 6 13 54 0 0 27 0 4 12 0 2 3 0 3 0 0 12 0 0 0 0 0 0
	bytes:	6917 0 0 0 6452 0 6917 6452
	RPC iostats version: 1.0  p/v: 100003/4 (nfs)
	xprt:	tcp 832 0 1 0 11 13 13 0 13 0 0 0 0
	per-op statistics
	        NULL: 0 0 0 0 0 0 0 0 0
	   COMPOUND: 27 27 0 1080 6156 6 51 57 0
	device server2:/export2 mounted on /mnt/nfs2 with fstype nfs4 statvers=1.1
	opts:	rw,vers=4.1,rsize=1048576,wsize=1048576,namlen=255,hard,proto=tcp,timeo=600,retrans=2
	age:	1800
	events:	10 20 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25
	bytes:	2048 0 0 0 1024 0 2048 1024
	per-op statistics
	        READ: 5 5 0 1000 2000 10 20 15 0
	       WRITE: 3 3 0 500 1500 5 15 10 1`

	tmpfile, err := ioutil.TempFile("", "mountstats_test")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(tmpfile.Name())

	if _, err := tmpfile.Write([]byte(mountstatsData)); err != nil {
		t.Fatal(err)
	}
	if err := tmpfile.Close(); err != nil {
		t.Fatal(err)
	}

	// Test parsing the file
	mounts, err := parseMountstats(tmpfile.Name())
	if err != nil {
		t.Errorf("parseMountstats() unexpected error: %v", err)
		return
	}

	// Verify we got the expected number of mounts
	expectedMounts := 2
	if len(mounts) != expectedMounts {
		t.Errorf("parseMountstats() got %d mounts, expected %d", len(mounts), expectedMounts)
	}

	// Check first mount
	mount1, exists := mounts["/mnt/nfs1"]
	if !exists {
		t.Error("parseMountstats() missing /mnt/nfs1 mount")
		return
	}

	if mount1.Device != "server1:/export1" {
		t.Errorf("mount1.Device = %v, want server1:/export1", mount1.Device)
	}
	if mount1.Server != "server1" {
		t.Errorf("mount1.Server = %v, want server1", mount1.Server)
	}
	if mount1.Export != "/export1" {
		t.Errorf("mount1.Export = %v, want /export1", mount1.Export)
	}
	if mount1.Age != 3661 {
		t.Errorf("mount1.Age = %v, want 3661", mount1.Age)
	}

	// Check events parsing
	if mount1.Events.InodeRevalidate != 27 {
		t.Errorf("mount1.Events.InodeRevalidate = %v, want 27", mount1.Events.InodeRevalidate)
	}

	// Check second mount
	mount2, exists := mounts["/mnt/nfs2"]
	if !exists {
		t.Error("parseMountstats() missing /mnt/nfs2 mount")
		return
	}

	if mount2.Age != 1800 {
		t.Errorf("mount2.Age = %v, want 1800", mount2.Age)
	}

	// Check operations parsing
	readOp, exists := mount2.Operations["READ"]
	if !exists {
		t.Error("parseMountstats() missing READ operation in mount2")
		return
	}
	if readOp.Ops != 5 {
		t.Errorf("readOp.Ops = %v, want 5", readOp.Ops)
	}
	if readOp.BytesSent != 1000 {
		t.Errorf("readOp.BytesSent = %v, want 1000", readOp.BytesSent)
	}

	writeOp, exists := mount2.Operations["WRITE"]
	if !exists {
		t.Error("parseMountstats() missing WRITE operation in mount2")
		return
	}
	if writeOp.Errors != 1 {
		t.Errorf("writeOp.Errors = %v, want 1", writeOp.Errors)
	}
}

// Test edge cases for initFlags
func TestInitFlagsEdgeCases(t *testing.T) {
	// Save original args and command line
	oldArgs := os.Args
	oldCommandLine := flag.CommandLine
	defer func() {
		os.Args = oldArgs
		flag.CommandLine = oldCommandLine
	}()

	t.Run("with positional arguments", func(t *testing.T) {
		// Reset flag state
		flag.CommandLine = flag.NewFlagSet(os.Args[0], flag.ContinueOnError)
		os.Args = []string{"nfs-gaze", "/mnt/test", "5", "10"}

		mountPoint, _, interval, count, _, _, _, _, _ := initFlags()

		if *mountPoint != "/mnt/test" {
			t.Errorf("mountPoint = %v, want /mnt/test", *mountPoint)
		}
		if *interval != 5*time.Second {
			t.Errorf("interval = %v, want 5s", *interval)
		}
		if *count != 10 {
			t.Errorf("count = %v, want 10", *count)
		}
	})

	t.Run("custom mountstats path", func(t *testing.T) {
		// Reset flag state
		flag.CommandLine = flag.NewFlagSet(os.Args[0], flag.ContinueOnError)
		os.Args = []string{"nfs-gaze", "-f", "/custom/mountstats"}

		_, _, _, _, _, _, _, _, mountstatsPath := initFlags()

		if *mountstatsPath != "/custom/mountstats" {
			t.Errorf("mountstatsPath = %v, want /custom/mountstats", *mountstatsPath)
		}
	})
}

// Test DisplayStatsNfsiostat with various scenarios
func TestDisplayStatsNfsiostatScenarios(t *testing.T) {
	mount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
		Events:     &NFSEvents{VFSOpen: 100, InodeRevalidate: 50, DataInvalidate: 10, AttrInvalidate: 5},
	}

	previousMount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
		Events:     &NFSEvents{VFSOpen: 80, InodeRevalidate: 30, DataInvalidate: 8, AttrInvalidate: 2},
	}

	stats := []*DeltaStats{
		{
			Operation:    "READ",
			DeltaOps:     100,
			DeltaRetrans: 5,
			DeltaErrors:  2,
			IOPS:         50.0,
			KBPerSec:     100.0,
			KBPerOp:      2.0,
			AvgRTT:       10.0,
			AvgExec:      5.0,
			AvgQueue:     2.0,
		},
	}

	t.Run("with attribute stats", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("displayStatsNfsiostat() with attr panicked: %v", r)
			}
		}()
		
		displayStatsNfsiostat(mount, stats, previousMount, true)
	})

	t.Run("with empty stats", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("displayStatsNfsiostat() with empty stats panicked: %v", r)
			}
		}()
		
		displayStatsNfsiostat(mount, []*DeltaStats{}, nil, false)
	})

	t.Run("with nil stats", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("displayStatsNfsiostat() with nil stats panicked: %v", r)
			}
		}()
		
		statsWithNil := []*DeltaStats{nil, stats[0]}
		displayStatsNfsiostat(mount, statsWithNil, nil, false)
	})
}

// Test DisplayStatsSimple with various scenarios
func TestDisplayStatsSimpleScenarios(t *testing.T) {
	mount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
	}

	stats := []*DeltaStats{
		{
			Operation: "READ",
			DeltaOps:  100,
			IOPS:      50.0,
			AvgRTT:    10.0,
			AvgExec:   5.0,
			KBPerSec:  100.0,
			KBPerOp:   2.0,
		},
		{
			Operation: "WRITE", 
			DeltaOps:  50,
			IOPS:      25.0,
			AvgRTT:    15.0,
			AvgExec:   8.0,
			KBPerSec:  200.0,
			KBPerOp:   4.0,
		},
	}

	t.Run("with bandwidth", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("displayStatsSimple() with bandwidth panicked: %v", r)
			}
		}()
		
		displayStatsSimple(mount, stats, true, time.Now())
	})

	t.Run("with nil stats", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("displayStatsSimple() with nil stats panicked: %v", r)
			}
		}()
		
		statsWithNil := []*DeltaStats{nil, stats[0]}
		displayStatsSimple(mount, statsWithNil, false, time.Now())
	})

	t.Run("with zero operations", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("displayStatsSimple() with zero ops panicked: %v", r)
			}
		}()
		
		zeroStats := []*DeltaStats{
			{Operation: "READ", DeltaOps: 0},
		}
		displayStatsSimple(mount, zeroStats, false, time.Now())
	})
}

// Test PrintInitialSummary edge cases  
func TestPrintInitialSummaryEdgeCases(t *testing.T) {
	// Test with zero operations
	mount := &NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
		Age:        3600,
		Operations: map[string]*NFSOperation{
			"READ": {
				Name: "READ",
				Ops:  0, // Zero operations
			},
		},
		Events: &NFSEvents{},
	}

	previousMounts := map[string]*NFSMount{"/mnt/nfs": mount}
	monitorMounts := []string{"/mnt/nfs"}
	opsFilter := map[string]bool{"READ": true}

	t.Run("nfsiostat mode with zero ops", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("printInitialSummary() with zero ops panicked: %v", r)
			}
		}()
		
		printInitialSummary(true, monitorMounts, previousMounts, opsFilter, false, "", 1*time.Second)
	})

	// Test with nil mount
	t.Run("nfsiostat mode with nil mount", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("printInitialSummary() with nil mount panicked: %v", r)
			}
		}()
		
		previousMountsWithNil := map[string]*NFSMount{"/mnt/nfs": nil}
		printInitialSummary(true, monitorMounts, previousMountsWithNil, opsFilter, false, "", 1*time.Second)
	})

	// Test with filtered operations
	t.Run("nfsiostat mode with filtered ops", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("printInitialSummary() with filtered ops panicked: %v", r)
			}
		}()
		
		// Filter that excludes READ operations
		filteredOpsFilter := map[string]bool{"WRITE": true}
		printInitialSummary(true, monitorMounts, previousMounts, filteredOpsFilter, false, "", 1*time.Second)
	})
}