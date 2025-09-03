//go:build linux

package tests

import (
	"flag"
	"os"
	"reflect"
	"testing"
	"time"

	internal "nfs-gazer/internal"
)

func TestParseOperationsFilter(t *testing.T) {
	tests := []struct {
		name       string
		operations string
		expected   map[string]bool
	}{
		{
			name:       "empty string",
			operations: "",
			expected:   nil,
		},
		{
			name:       "single operation",
			operations: "READ",
			expected:   map[string]bool{"READ": true},
		},
		{
			name:       "multiple operations",
			operations: "READ,WRITE,GETATTR",
			expected:   map[string]bool{"READ": true, "WRITE": true, "GETATTR": true},
		},
		{
			name:       "operations with spaces",
			operations: "READ, WRITE , GETATTR",
			expected:   map[string]bool{"READ": true, "WRITE": true, "GETATTR": true},
		},
		{
			name:       "single operation with spaces",
			operations: " READ ",
			expected:   map[string]bool{"READ": true},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := internal.ParseOperationsFilter(tt.operations)
			
			if !reflect.DeepEqual(result, tt.expected) {
				t.Errorf("internal.ParseOperationsFilter() = %v, want %v", result, tt.expected)
			}
		})
	}
}

func TestGetMountsToMonitor(t *testing.T) {
	previousMounts := map[string]*internal.NFSMount{
		"/mnt/nfs1": {MountPoint: "/mnt/nfs1", Device: "server1:/export1"},
		"/mnt/nfs2": {MountPoint: "/mnt/nfs2", Device: "server2:/export2"},
	}

	tests := []struct {
		name           string
		mountPoint     string
		previousMounts map[string]*internal.NFSMount
		expected       []string
		shouldExit     bool
	}{
		{
			name:           "specific mount point",
			mountPoint:     "/mnt/nfs1",
			previousMounts: previousMounts,
			expected:       []string{"/mnt/nfs1"},
			shouldExit:     false,
		},
		{
			name:           "all mount points",
			mountPoint:     "",
			previousMounts: previousMounts,
			expected:       []string{"/mnt/nfs1", "/mnt/nfs2"}, // order may vary
			shouldExit:     false,
		},
		{
			name:           "empty mounts map",
			mountPoint:     "",
			previousMounts: map[string]*internal.NFSMount{},
			expected:       nil,
			shouldExit:     true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.shouldExit {
				// Skip tests that would call os.Exit(1)
				t.Skip("Test would call os.Exit(1)")
				return
			}

			result := internal.GetMountsToMonitor(tt.mountPoint, tt.previousMounts)
			
			if tt.mountPoint == "" && len(tt.previousMounts) > 0 {
				// For empty mount point, check that we got all mounts
				if len(result) != len(tt.previousMounts) {
					t.Errorf("internal.GetMountsToMonitor() returned %d mounts, expected %d", len(result), len(tt.previousMounts))
				}
				// Check that all expected mount points are present
				mountMap := make(map[string]bool)
				for _, mp := range result {
					mountMap[mp] = true
				}
				for expectedMP := range tt.previousMounts {
					if !mountMap[expectedMP] {
						t.Errorf("internal.GetMountsToMonitor() missing expected mount point: %s", expectedMP)
					}
				}
			} else {
				if !reflect.DeepEqual(result, tt.expected) {
					t.Errorf("internal.GetMountsToMonitor() = %v, want %v", result, tt.expected)
				}
			}
		})
	}
}

func TestInitFlags(t *testing.T) {
	// Save original command line args and flags
	oldArgs := os.Args
	oldCommandLine := flag.CommandLine
	defer func() {
		os.Args = oldArgs
		flag.CommandLine = oldCommandLine
	}()

	tests := []struct {
		name     string
		args     []string
		expected map[string]interface{}
	}{
		{
			name: "default values",
			args: []string{"nfs-gaze"},
			expected: map[string]interface{}{
				"mountPoint":     "",
				"operations":     "",
				"interval":       1 * time.Second,
				"count":          0,
				"showAttr":       false,
				"showBandwidth":  false,
				"nfsiostatMode":  false,
				"clearScreen":    false,
				"mountstatsPath": "/proc/self/mountstats",
			},
		},
		{
			name: "with flags",
			args: []string{"nfs-gaze", "-m", "/mnt/nfs", "-ops", "READ,WRITE", "-i", "2s", "-c", "5", "-attr", "-bw", "-nfsiostat", "-clear"},
			expected: map[string]interface{}{
				"mountPoint":     "/mnt/nfs",
				"operations":     "READ,WRITE",
				"interval":       2 * time.Second,
				"count":          5,
				"showAttr":       true,
				"showBandwidth":  true,
				"nfsiostatMode":  true,
				"clearScreen":    true,
				"mountstatsPath": "/proc/self/mountstats",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Reset flag state
			flag.CommandLine = flag.NewFlagSet(os.Args[0], flag.ContinueOnError)
			os.Args = tt.args

			mountPoint, operations, interval, count, showAttr, showBandwidth, nfsiostatMode, clearScreen, mountstatsPath := internal.InitFlags()

			if *mountPoint != tt.expected["mountPoint"] {
				t.Errorf("mountPoint = %v, want %v", *mountPoint, tt.expected["mountPoint"])
			}
			if *operations != tt.expected["operations"] {
				t.Errorf("operations = %v, want %v", *operations, tt.expected["operations"])
			}
			if *interval != tt.expected["interval"] {
				t.Errorf("interval = %v, want %v", *interval, tt.expected["interval"])
			}
			if *count != tt.expected["count"] {
				t.Errorf("count = %v, want %v", *count, tt.expected["count"])
			}
			if *showAttr != tt.expected["showAttr"] {
				t.Errorf("showAttr = %v, want %v", *showAttr, tt.expected["showAttr"])
			}
			if *showBandwidth != tt.expected["showBandwidth"] {
				t.Errorf("showBandwidth = %v, want %v", *showBandwidth, tt.expected["showBandwidth"])
			}
			if *nfsiostatMode != tt.expected["nfsiostatMode"] {
				t.Errorf("nfsiostatMode = %v, want %v", *nfsiostatMode, tt.expected["nfsiostatMode"])
			}
			if *clearScreen != tt.expected["clearScreen"] {
				t.Errorf("clearScreen = %v, want %v", *clearScreen, tt.expected["clearScreen"])
			}
			if *mountstatsPath != tt.expected["mountstatsPath"] {
				t.Errorf("mountstatsPath = %v, want %v", *mountstatsPath, tt.expected["mountstatsPath"])
			}
		})
	}
}

func TestPrintInitialSummary(t *testing.T) {
	mount := &internal.NFSMount{
		Device:     "server:/export",
		MountPoint: "/mnt/nfs",
		Age:        3600,
		Operations: map[string]*internal.NFSOperation{
			"READ": {
				Name:        "READ",
				Ops:         100,
				BytesSent:   1000,
				BytesRecv:   5000,
				RTT:         500,
				ExecuteTime: 200,
				QueueTime:   100,
			},
		},
		Events: &internal.NFSEvents{},
	}

	previousMounts := map[string]*internal.NFSMount{
		"/mnt/nfs": mount,
	}
	monitorMounts := []string{"/mnt/nfs"}
	opsFilter := map[string]bool{"READ": true}

	// Test that the function doesn't panic
	t.Run("nfsiostat mode no panic", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("internal.PrintInitialSummary() panicked: %v", r)
			}
		}()
		
		internal.PrintInitialSummary(true, monitorMounts, previousMounts, opsFilter, false, "READ", 1*time.Second)
	})

	t.Run("simple mode no panic", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("internal.PrintInitialSummary() panicked: %v", r)
			}
		}()
		
		internal.PrintInitialSummary(false, monitorMounts, previousMounts, opsFilter, false, "READ", 1*time.Second)
	})
}