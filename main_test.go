//go:build linux

package main

import (
	"math"
	"os"
	"reflect"
	"sort"
	"testing"
)

func TestParseEvents(t *testing.T) {
	tests := []struct {
		name    string
		parts   []string
		want    *NFSEvents
		wantErr bool
	}{
		{
			name:  "valid events",
			parts: []string{"1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27"},
			want: &NFSEvents{
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
			name:    "invalid events",
			parts:   []string{"1", "2"},
			want:    &NFSEvents{},
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
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("parseEvents() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestCalculateDelta(t *testing.T) {
	type args struct {
		previousOp  *NFSOperation
		currentOp   *NFSOperation
		durationSec float64
	}
	tests := []struct {
		name string
		args args
		want *DeltaStats
	}{
		{
			name: "valid delta",
			args: args{
				previousOp: &NFSOperation{
					Name:        "READ",
					Ops:         10,
					BytesSent:   100,
					BytesRecv:   200,
					RTT:         300,
					ExecuteTime: 400,
					QueueTime:   500,
					Timeouts:    600,
					Errors:      700,
				},
				currentOp: &NFSOperation{
					Name:        "READ",
					Ops:         20,
					BytesSent:   200,
					BytesRecv:   400,
					RTT:         600,
					ExecuteTime: 800,
					QueueTime:   1000,
					Timeouts:    1200,
					Errors:      1400,
				},
				durationSec: 1,
			},
			want: &DeltaStats{
				Operation:    "READ",
				DeltaOps:     10,
				DeltaSent:    100,
				DeltaRecv:    200,
				DeltaBytes:   300,
				DeltaRTT:     300,
				DeltaExec:    400,
				DeltaQueue:   500,
				DeltaRetrans: 600,
				DeltaErrors:  700,
				IOPS:         10,
				AvgRTT:       30,
				AvgExec:      40,
				AvgQueue:     50,
				KBPerOp:      0.029296875,
				KBPerSec:     0.29296875,
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := calculateDelta(tt.args.previousOp, tt.args.currentOp, tt.args.durationSec)
			if math.Abs(got.KBPerOp-tt.want.KBPerOp) > 0.000001 {
				t.Errorf("calculateDelta() KBPerOp = %v, want %v", got.KBPerOp, tt.want.KBPerOp)
			}
			if math.Abs(got.KBPerSec-tt.want.KBPerSec) > 0.000001 {
				t.Errorf("calculateDelta() KBPerSec = %v, want %v", got.KBPerSec, tt.want.KBPerSec)
			}
			got.KBPerOp = 0
			tt.want.KBPerOp = 0
			got.KBPerSec = 0
			tt.want.KBPerSec = 0
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("calculateDelta() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestGetMountsToMonitor(t *testing.T) {
	tests := []struct {
		name          string
		mountPoint    string
		previousMounts map[string]*NFSMount
		want          []string
		wantErr       bool
	}{
		{
			name:       "mount point found",
			mountPoint: "/mnt/nfs",
			previousMounts: map[string]*NFSMount{
				"/mnt/nfs": {},
			},
			want:    []string{"/mnt/nfs"},
			wantErr: false,
		},
		{
			name:       "mount point not found",
			mountPoint: "/mnt/nfs2",
			previousMounts: map[string]*NFSMount{
				"/mnt/nfs": {},
			},
			want:    nil,
			wantErr: true,
		},
		{
			name:       "no mount point specified",
			mountPoint: "",
			previousMounts: map[string]*NFSMount{
				"/mnt/nfs1": {},
				"/mnt/nfs2": {},
			},
			want:    []string{"/mnt/nfs1", "/mnt/nfs2"},
			wantErr: false,
		},
		{
			name:          "no nfs mounts found",
			mountPoint:    "",
			previousMounts: map[string]*NFSMount{},
			want:          nil,
			wantErr:       true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := getMountsToMonitor(tt.mountPoint, tt.previousMounts)
			if (err != nil) != tt.wantErr {
				t.Errorf("getMountsToMonitor() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			// sort the slices to ensure they are in the same order
			sort.Strings(got)
			sort.Strings(tt.want)
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("getMountsToMonitor() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestParseMountstats(t *testing.T) {
	tests := []struct {
		name    string
		content string
		want    map[string]*NFSMount
		wantErr bool
	}{
		{
			name: "valid mountstats",
			content: `device 10.0.0.1:/mnt/nfs on /mnt/nfs type nfs4 (ro,relatime,vers=4.2,rsize=1048576,wsize=1048576,namlen=255,hard,proto=tcp,timeo=600,retrans=2,sec=sys,clientaddr=10.0.0.2,local_lock=none,addr=10.0.0.1) statvers=1.1
age: 12345
events: 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27
bytes: 100 0 0 0 200 0 0 0
READ: 10 1 1 100 200 500 300 400 700
WRITE: 0 0 0 0 0 0 0 0 0
`,
			want: map[string]*NFSMount{
				"/mnt/nfs": {
					Device:     "10.0.0.1:/mnt/nfs",
					MountPoint: "/mnt/nfs",
					Server:     "10.0.0.1",
					Export:     "/mnt/nfs",
					Age:        12345,
					Events: &NFSEvents{
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
					BytesRead:  100,
					BytesWrite: 200,
					Operations: map[string]*NFSOperation{
						"READ": {
							Name:        "READ",
							Ops:         10,
							Ntrans:      1,
							Timeouts:    1,
							BytesSent:   100,
							BytesRecv:   200,
							QueueTime:   500,
							RTT:         300,
							ExecuteTime: 400,
							Errors:      700,
						},
						"WRITE": {
							Name:        "WRITE",
							Ops:         0,
							Ntrans:      0,
							Timeouts:    0,
							BytesSent:   0,
							BytesRecv:   0,
							QueueTime:   0,
							RTT:         0,
							ExecuteTime: 0,
							Errors:      0,
						},
					},
				},
			},
			wantErr: false,
		},
		{
			name:    "malformed device line",
			content: `device 10.0.0.1:/mnt/nfs type nfs4`,
			want:    map[string]*NFSMount{},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tmpfile, err := os.CreateTemp("", "mountstats")
			if err != nil {
				t.Fatal(err)
			}
			defer os.Remove(tmpfile.Name())

			if _, err := tmpfile.Write([]byte(tt.content)); err != nil {
				t.Fatal(err)
			}
			if err := tmpfile.Close(); err != nil {
				t.Fatal(err)
			}

			got, err := parseMountstats(tmpfile.Name())
			if (err != nil) != tt.wantErr {
				t.Errorf("parseMountstats() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("parseMountstats() = %v, want %v", got, tt.want)
			}
		})
	}
}