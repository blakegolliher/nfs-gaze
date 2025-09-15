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
				NfsiostatMode:  false,
				ClearScreen:    false,
				MountstatsPath: "/proc/self/mountstats",
			},
		},
		{
			name: "with mount point flag",
			args: []string{"nfs-gaze", "-m", "/mnt/nfs"},
			expected: &Flags{
				MountPoint:     "/mnt/nfs",
				Operations:     "",
				Interval:       1 * time.Second,
				Count:          0,
				ShowAttr:       false,
				ShowBandwidth:  false,
				NfsiostatMode:  false,
				ClearScreen:    false,
				MountstatsPath: "/proc/self/mountstats",
			},
		},
		{
			name: "with multiple flags",
			args: []string{"nfs-gaze", "-m", "/mnt/nfs", "-ops", "READ,WRITE", "-i", "5s", "-c", "10", "--attr", "--bw"},
			expected: &Flags{
				MountPoint:     "/mnt/nfs",
				Operations:     "READ,WRITE",
				Interval:       5 * time.Second,
				Count:          10,
				ShowAttr:       true,
				ShowBandwidth:  true,
				NfsiostatMode:  false,
				ClearScreen:    false,
				MountstatsPath: "/proc/self/mountstats",
			},
		},
		{
			name: "positional arguments",
			args: []string{"nfs-gaze", "/mnt/nfs", "2", "5"},
			expected: &Flags{
				MountPoint:     "/mnt/nfs",
				Operations:     "",
				Interval:       2 * time.Second,
				Count:          5,
				ShowAttr:       false,
				ShowBandwidth:  false,
				NfsiostatMode:  false,
				ClearScreen:    false,
				MountstatsPath: "/proc/self/mountstats",
			},
		},
		{
			name: "nfsiostat mode",
			args: []string{"nfs-gaze", "--nfsiostat", "-m", "/mnt/nfs"},
			expected: &Flags{
				MountPoint:     "/mnt/nfs",
				Operations:     "",
				Interval:       1 * time.Second,
				Count:          0,
				ShowAttr:       false,
				ShowBandwidth:  false,
				NfsiostatMode:  true,
				ClearScreen:    false,
				MountstatsPath: "/proc/self/mountstats",
			},
		},
		{
			name: "custom mountstats path",
			args: []string{"nfs-gaze", "-f", "/custom/path/mountstats"},
			expected: &Flags{
				MountPoint:     "",
				Operations:     "",
				Interval:       1 * time.Second,
				Count:          0,
				ShowAttr:       false,
				ShowBandwidth:  false,
				NfsiostatMode:  false,
				ClearScreen:    false,
				MountstatsPath: "/custom/path/mountstats",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Reset flag.CommandLine for each test
			flag.CommandLine = flag.NewFlagSet(os.Args[0], flag.ContinueOnError)
			os.Args = tt.args

			got := initFlags()

			if !reflect.DeepEqual(got, tt.expected) {
				t.Errorf("initFlags() = %+v, want %+v", got, tt.expected)
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
			got := parseOperationsFilter(tt.operations)
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("parseOperationsFilter(%q) = %v, want %v", tt.operations, got, tt.want)
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
				"/mnt/nfs": {MountPoint: "/mnt/nfs"},
			},
			want:    []string{"/mnt/nfs"},
			wantErr: false,
		},
		{
			name:       "specific mount point does not exist",
			mountPoint: "/mnt/nonexistent",
			previousMounts: map[string]*NFSMount{
				"/mnt/nfs": {MountPoint: "/mnt/nfs"},
			},
			want:    nil,
			wantErr: true,
		},
		{
			name:       "all mounts when no specific mount point",
			mountPoint: "",
			previousMounts: map[string]*NFSMount{
				"/mnt/nfs1": {MountPoint: "/mnt/nfs1"},
				"/mnt/nfs2": {MountPoint: "/mnt/nfs2"},
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
			got, err := getMountsToMonitor(tt.mountPoint, tt.previousMounts)
			if (err != nil) != tt.wantErr {
				t.Errorf("getMountsToMonitor() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr {
				// For the "all mounts" case, we need to handle unordered results
				if tt.mountPoint == "" && len(got) == len(tt.want) {
					gotMap := make(map[string]bool)
					for _, m := range got {
						gotMap[m] = true
					}
					for _, w := range tt.want {
						if !gotMap[w] {
							t.Errorf("getMountsToMonitor() missing mount %s", w)
						}
					}
				} else if !reflect.DeepEqual(got, tt.want) {
					t.Errorf("getMountsToMonitor() = %v, want %v", got, tt.want)
				}
			}
		})
	}
}

func TestPrintInitialSummary(t *testing.T) {
	// This test primarily ensures the function doesn't panic
	// Actual output testing would require capturing stdout which is more complex

	flags := &Flags{
		NfsiostatMode: false,
		Interval:      1 * time.Second,
		Operations:    "READ,WRITE",
		ShowAttr:      false,
	}

	monitorMounts := []string{"/mnt/nfs"}

	previousMounts := map[string]*NFSMount{
		"/mnt/nfs": {
			Device:     "server:/export",
			MountPoint: "/mnt/nfs",
			Age:        3600,
			Operations: map[string]*NFSOperation{
				"READ": {
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

	// Test non-nfsiostat mode
	t.Run("simple mode", func(t *testing.T) {
		// Redirect stdout to prevent test output pollution
		oldStdout := os.Stdout
		os.Stdout, _ = os.Open(os.DevNull)
		defer func() { os.Stdout = oldStdout }()

		// Should not panic
		printInitialSummary(flags, monitorMounts, previousMounts, opsFilter)
	})

	// Test nfsiostat mode
	t.Run("nfsiostat mode", func(t *testing.T) {
		nfsFlags := *flags
		nfsFlags.NfsiostatMode = true

		// Redirect stdout to prevent test output pollution
		oldStdout := os.Stdout
		os.Stdout, _ = os.Open(os.DevNull)
		defer func() { os.Stdout = oldStdout }()

		// Should not panic
		printInitialSummary(&nfsFlags, monitorMounts, previousMounts, opsFilter)
	})
}