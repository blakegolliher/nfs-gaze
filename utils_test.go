package main

import (
	"flag"
	"os"
	"reflect"
	"testing"
	"time"
)

func TestInitFlags(t *testing.T) {
	// Save original command-line arguments
	oldArgs := os.Args
	oldCommandLine := flag.CommandLine
	defer func() {
		os.Args = oldArgs
		flag.CommandLine = oldCommandLine
	}()

	tests := []struct {
		name     string
		args     []string
		expected *Flags
	}{
		{
			name: "default flags",
			args: []string{"nfs-gaze"},
			expected: &Flags{
				MountPoint:     "",
				Operations:     "",
				Interval:       1 * time.Second,
				Count:          0,
				ShowAttr:       false,
				ShowBandwidth:  false,
				ClearScreen:    false,
				MountstatsPath: "/proc/self/mountstats",
			},
		},
		{
			name: "with mount point",
			args: []string{"nfs-gaze", "-m", "/mnt/nfs"},
			expected: &Flags{
				MountPoint:     "/mnt/nfs",
				Operations:     "",
				Interval:       1 * time.Second,
				Count:          0,
				ShowAttr:       false,
				ShowBandwidth:  false,
				ClearScreen:    false,
				MountstatsPath: "/proc/self/mountstats",
			},
		},
		{
			name: "with operations filter",
			args: []string{"nfs-gaze", "-ops", "READ,WRITE"},
			expected: &Flags{
				MountPoint:     "",
				Operations:     "READ,WRITE",
				Interval:       1 * time.Second,
				Count:          0,
				ShowAttr:       false,
				ShowBandwidth:  false,
				ClearScreen:    false,
				MountstatsPath: "/proc/self/mountstats",
			},
		},
		{
			name: "with custom interval",
			args: []string{"nfs-gaze", "-i", "5s"},
			expected: &Flags{
				MountPoint:     "",
				Operations:     "",
				Interval:       5 * time.Second,
				Count:          0,
				ShowAttr:       false,
				ShowBandwidth:  false,
				ClearScreen:    false,
				MountstatsPath: "/proc/self/mountstats",
			},
		},
		{
			name: "with all flags",
			args: []string{"nfs-gaze", "-m", "/mnt/nfs", "-ops", "READ", "-i", "2s", "-c", "10", "-attr", "-bw", "-clear"},
			expected: &Flags{
				MountPoint:     "/mnt/nfs",
				Operations:     "READ",
				Interval:       2 * time.Second,
				Count:          10,
				ShowAttr:       true,
				ShowBandwidth:  true,
				ClearScreen:    true,
				MountstatsPath: "/proc/self/mountstats",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Reset flag.CommandLine for each test
			flag.CommandLine = flag.NewFlagSet(os.Args[0], flag.ContinueOnError)
			os.Args = tt.args

			got := InitFlags()

			if !reflect.DeepEqual(got, tt.expected) {
				t.Errorf("InitFlags() = %+v, want %+v", got, tt.expected)
			}
		})
	}
}

func TestParseOperationsFilter(t *testing.T) {
	tests := []struct {
		name       string
		operations string
		want       map[string]bool
	}{
		{
			name:       "empty string",
			operations: "",
			want:       nil,
		},
		{
			name:       "single operation",
			operations: "READ",
			want:       map[string]bool{"READ": true},
		},
		{
			name:       "multiple operations",
			operations: "READ,WRITE,GETATTR",
			want:       map[string]bool{"READ": true, "WRITE": true, "GETATTR": true},
		},
		{
			name:       "operations with spaces",
			operations: "READ, WRITE , GETATTR",
			want:       map[string]bool{"READ": true, "WRITE": true, "GETATTR": true},
		},
		{
			name:       "duplicate operations",
			operations: "READ,READ,WRITE",
			want:       map[string]bool{"READ": true, "WRITE": true},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := ParseOperationsFilter(tt.operations)
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("ParseOperationsFilter(%q) = %v, want %v", tt.operations, got, tt.want)
			}
		})
	}
}

func TestGetMountsToMonitor(t *testing.T) {
	tests := []struct {
		name           string
		mountPoint     string
		previousMounts map[string]*NFSMount
		want           []string
		wantErr        bool
	}{
		{
			name:       "specific mount point exists",
			mountPoint: "/mnt/nfs",
			previousMounts: map[string]*NFSMount{
				"/mnt/nfs": &NFSMount{MountPoint: "/mnt/nfs"},
			},
			want:    []string{"/mnt/nfs"},
			wantErr: false,
		},
		{
			name:       "specific mount point does not exist",
			mountPoint: "/mnt/nonexistent",
			previousMounts: map[string]*NFSMount{
				"/mnt/nfs": &NFSMount{MountPoint: "/mnt/nfs"},
			},
			want:    nil,
			wantErr: true,
		},
		{
			name:       "all mounts when no specific mount point",
			mountPoint: "",
			previousMounts: map[string]*NFSMount{
				"/mnt/nfs1": &NFSMount{MountPoint: "/mnt/nfs1"},
				"/mnt/nfs2": &NFSMount{MountPoint: "/mnt/nfs2"},
			},
			want:    []string{"/mnt/nfs1", "/mnt/nfs2"},
			wantErr: false,
		},
		{
			name:           "no mounts available",
			mountPoint:     "",
			previousMounts: map[string]*NFSMount{},
			want:           nil,
			wantErr:        true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := GetMountsToMonitor(tt.mountPoint, tt.previousMounts)
			if (err != nil) != tt.wantErr {
				t.Errorf("GetMountsToMonitor() error = %v, wantErr %v", err, tt.wantErr)
				return
			}

			// Sort both slices for comparison since order may vary
			if len(got) != len(tt.want) {
				t.Errorf("GetMountsToMonitor() = %v, want %v", got, tt.want)
				return
			}

			gotMap := make(map[string]bool)
			for _, v := range got {
				gotMap[v] = true
			}
			for _, v := range tt.want {
				if !gotMap[v] {
					t.Errorf("GetMountsToMonitor() = %v, want %v", got, tt.want)
					return
				}
			}
		})
	}
}

func TestPrintInitialSummary(t *testing.T) {
	// This test mainly ensures that printInitialSummary doesn't panic

	flags := &Flags{
		Interval:      1 * time.Second,
		Operations:    "READ,WRITE",
		ShowAttr:      false,
	}

	monitorMounts := []string{"/mnt/nfs"}

	previousMounts := map[string]*NFSMount{
		"/mnt/nfs": &NFSMount{
			Device:     "server:/export",
			MountPoint: "/mnt/nfs",
			Age:        3600,
			Operations: map[string]*NFSOperation{
				"READ": &NFSOperation{
					Name:        "READ",
					Ops:         100,
					BytesSent:   1024,
					BytesRecv:   2048,
					RTT:         500,
					ExecuteTime: 1000,
					QueueTime:   100,
				},
			},
		},
	}

	opsFilter := map[string]bool{"READ": true, "WRITE": true}

	// Test that the function doesn't panic
	t.Run("simple mode", func(t *testing.T) {
		// Redirect stdout to prevent test output pollution
		oldStdout := os.Stdout
		os.Stdout, _ = os.Open(os.DevNull)
		defer func() { os.Stdout = oldStdout }()

		// Should not panic
		PrintInitialSummary(flags, monitorMounts, previousMounts, opsFilter)
	})
}