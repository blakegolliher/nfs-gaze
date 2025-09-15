//go:build linux

package main

import (
	"bufio"
	"fmt"
	"io"
	"log"
	"os"
	"strconv"
	"strings"
	"time"
)

// ParseEvents parses the events line into an NFSEvents struct.
func ParseEvents(parts []string) (*NFSEvents, error) {
	if len(parts) < 27 {
		return nil, fmt.Errorf("invalid number of parts for events: %d", len(parts))
	}

	parseInt := func(s string) (int64, error) {
		return strconv.ParseInt(s, 10, 64)
	}

	var (
		err error
		e   NFSEvents
	)

	if e.InodeRevalidate, err = parseInt(parts[0]); err != nil {
		return nil, fmt.Errorf("error parsing InodeRevalidate: %w", err)
	}
	if e.DentryRevalidate, err = parseInt(parts[1]); err != nil {
		return nil, fmt.Errorf("error parsing DentryRevalidate: %w", err)
	}
	if e.DataInvalidate, err = parseInt(parts[2]); err != nil {
		return nil, fmt.Errorf("error parsing DataInvalidate: %w", err)
	}
	if e.AttrInvalidate, err = parseInt(parts[3]); err != nil {
		return nil, fmt.Errorf("error parsing AttrInvalidate: %w", err)
	}
	if e.VFSOpen, err = parseInt(parts[4]); err != nil {
		return nil, fmt.Errorf("error parsing VFSOpen: %w", err)
	}
	if e.VFSLookup, err = parseInt(parts[5]); err != nil {
		return nil, fmt.Errorf("error parsing VFSLookup: %w", err)
	}
	if e.VFSAccess, err = parseInt(parts[6]); err != nil {
		return nil, fmt.Errorf("error parsing VFSAccess: %w", err)
	}
	if e.VFSUpdatePage, err = parseInt(parts[7]); err != nil {
		return nil, fmt.Errorf("error parsing VFSUpdatePage: %w", err)
	}
	if e.VFSReadPage, err = parseInt(parts[8]); err != nil {
		return nil, fmt.Errorf("error parsing VFSReadPage: %w", err)
	}
	if e.VFSReadPages, err = parseInt(parts[9]); err != nil {
		return nil, fmt.Errorf("error parsing VFSReadPages: %w", err)
	}
	if e.VFSWritePage, err = parseInt(parts[10]); err != nil {
		return nil, fmt.Errorf("error parsing VFSWritePage: %w", err)
	}
	if e.VFSWritePages, err = parseInt(parts[11]); err != nil {
		return nil, fmt.Errorf("error parsing VFSWritePages: %w", err)
	}
	if e.VFSGetdents, err = parseInt(parts[12]); err != nil {
		return nil, fmt.Errorf("error parsing VFSGetdents: %w", err)
	}
	if e.VFSSetattr, err = parseInt(parts[13]); err != nil {
		return nil, fmt.Errorf("error parsing VFSSetattr: %w", err)
	}
	if e.VFSFlush, err = parseInt(parts[14]); err != nil {
		return nil, fmt.Errorf("error parsing VFSFlush: %w", err)
	}
	if e.VFSFsync, err = parseInt(parts[15]); err != nil {
		return nil, fmt.Errorf("error parsing VFSFsync: %w", err)
	}
	if e.VFSLock, err = parseInt(parts[16]); err != nil {
		return nil, fmt.Errorf("error parsing VFSLock: %w", err)
	}
	if e.VFSRelease, err = parseInt(parts[17]); err != nil {
		return nil, fmt.Errorf("error parsing VFSRelease: %w", err)
	}
	if e.CongestionWait, err = parseInt(parts[18]); err != nil {
		return nil, fmt.Errorf("error parsing CongestionWait: %w", err)
	}
	if e.SetattrTrunc, err = parseInt(parts[19]); err != nil {
		return nil, fmt.Errorf("error parsing SetattrTrunc: %w", err)
	}
	if e.ExtendWrite, err = parseInt(parts[20]); err != nil {
		return nil, fmt.Errorf("error parsing ExtendWrite: %w", err)
	}
	if e.SillyRename, err = parseInt(parts[21]); err != nil {
		return nil, fmt.Errorf("error parsing SillyRename: %w", err)
	}
	if e.ShortRead, err = parseInt(parts[22]); err != nil {
		return nil, fmt.Errorf("error parsing ShortRead: %w", err)
	}
	if e.ShortWrite, err = parseInt(parts[23]); err != nil {
		return nil, fmt.Errorf("error parsing ShortWrite: %w", err)
	}
	if e.Delay, err = parseInt(parts[24]); err != nil {
		return nil, fmt.Errorf("error parsing Delay: %w", err)
	}
	if len(parts) > 25 {
		if e.PNFSRead, err = parseInt(parts[25]); err != nil {
			return nil, fmt.Errorf("error parsing PNFSRead: %w", err)
		}
	}
	if len(parts) > 26 {
		if e.PNFSWrite, err = parseInt(parts[26]); err != nil {
			return nil, fmt.Errorf("error parsing PNFSWrite: %w", err)
		}
	}

	return &e, nil
}

func ParseNFSOperation(opName string, stats []string) (*NFSOperation, error) {
	if len(stats) < 9 {
		return nil, fmt.Errorf("invalid number of stats for op %s: %d", opName, len(stats))
	}

	parseInt := func(s string) (int64, error) {
		return strconv.ParseInt(s, 10, 64)
	}

	var (
		err error
		op  NFSOperation
	)

	op.Name = opName
	if op.Ops, err = parseInt(stats[0]); err != nil {
		return nil, fmt.Errorf("error parsing ops for %s: %w", opName, err)
	}
	if op.Ntrans, err = parseInt(stats[1]); err != nil {
		return nil, fmt.Errorf("error parsing ntrans for %s: %w", opName, err)
	}
	if op.Timeouts, err = parseInt(stats[2]); err != nil {
		return nil, fmt.Errorf("error parsing timeouts for %s: %w", opName, err)
	}
	if op.BytesSent, err = parseInt(stats[3]); err != nil {
		return nil, fmt.Errorf("error parsing bytes sent for %s: %w", opName, err)
	}
	if op.BytesRecv, err = parseInt(stats[4]); err != nil {
		return nil, fmt.Errorf("error parsing bytes recv for %s: %w", opName, err)
	}
	if op.QueueTime, err = parseInt(stats[5]); err != nil {
		return nil, fmt.Errorf("error parsing queue time for %s: %w", opName, err)
	}
	if op.RTT, err = parseInt(stats[6]); err != nil {
		return nil, fmt.Errorf("error parsing rtt for %s: %w", opName, err)
	}
	if op.ExecuteTime, err = parseInt(stats[7]); err != nil {
		return nil, fmt.Errorf("error parsing execute time for %s: %w", opName, err)
	}
	if len(stats) > 8 {
		if op.Errors, err = parseInt(stats[8]); err != nil {
			return nil, fmt.Errorf("error parsing errors for %s: %w", opName, err)
		}
	}

	return &op, nil
}

type mountstatsParser struct {
	scanner      *bufio.Scanner
	mounts       map[string]*NFSMount
	currentMount *NFSMount
}

func (p *mountstatsParser) parse() error {
	for p.scanner.Scan() {
		if err := p.parseLine(p.scanner.Text()); err != nil {
			return err
		}
	}
	return p.scanner.Err()
}

func (p *mountstatsParser) parseLine(line string) error {
	line = strings.TrimSpace(line)
	if strings.HasPrefix(line, "device") && strings.Contains(line, "nfs") {
		return p.parseDeviceLine(line)
	} else if p.currentMount != nil {
		return p.parseStatsLine(line)
	}
	return nil
}

func (p *mountstatsParser) parseDeviceLine(line string) error {
	lineParts := strings.SplitN(line, " on ", 2)
	if len(lineParts) != 2 {
		return fmt.Errorf("invalid device line: %s", line)
	}
	deviceInfo := strings.Fields(lineParts[0])
	mountInfo := strings.Fields(lineParts[1])

	if len(deviceInfo) < 2 || len(mountInfo) < 1 {
		return fmt.Errorf("invalid device info: %s", line)
	}

	serverExport := deviceInfo[1]
	mountPoint := mountInfo[0]

	serverParts := strings.SplitN(serverExport, ":", 2)
	server := serverParts[0]
	export := "/"
	if len(serverParts) > 1 {
		export = serverParts[1]
	}

	p.currentMount = &NFSMount{
		Device:     serverExport,
		MountPoint: mountPoint,
		Server:     server,
		Export:     export,
		Operations: make(map[string]*NFSOperation),
		Events:     &NFSEvents{},
	}
	p.mounts[mountPoint] = p.currentMount
	return nil
}

func (p *mountstatsParser) parseStatsLine(line string) error {
	switch {
	case strings.HasPrefix(line, "age:"):
		return p.parseAge(line)
	case strings.HasPrefix(line, "events:"):
		return p.parseEvents(line)
	case strings.HasPrefix(line, "bytes:"):
		return p.parseBytes(line)
	case strings.Contains(line, ":") && !strings.HasPrefix(line, "RPC") &&
		!strings.HasPrefix(line, "xprt") && !strings.HasPrefix(line, "per-op") &&
		!strings.HasPrefix(line, "opts") && !strings.HasPrefix(line, "caps") &&
		!strings.HasPrefix(line, "sec") && !strings.HasPrefix(line, "nfsv4") &&
		!strings.HasPrefix(line, "nfsv3"):
		return p.parseOperation(line)
	}
	return nil
}

func (p *mountstatsParser) parseAge(line string) error {
	parts := strings.Fields(line)
	if len(parts) < 2 {
		return fmt.Errorf("invalid age line: %s", line)
	}
	var err error
	p.currentMount.Age, err = strconv.ParseInt(parts[1], 10, 64)
	return err
}

func (p *mountstatsParser) parseEvents(line string) error {
	parts := strings.Fields(line)
	if len(parts) < 2 {
		return fmt.Errorf("invalid events line: %s", line)
	}
	var err error
	p.currentMount.Events, err = ParseEvents(parts[1:])
	return err
}

func (p *mountstatsParser) parseBytes(line string) error {
	parts := strings.Fields(line)
	if len(parts) < 6 {
		return fmt.Errorf("invalid bytes line: %s", line)
	}
	var err error
	p.currentMount.BytesRead, err = strconv.ParseInt(parts[1], 10, 64)
	if err != nil {
		return err
	}
	p.currentMount.BytesWrite, err = strconv.ParseInt(parts[5], 10, 64)
	return err
}

func (p *mountstatsParser) parseOperation(line string) error {
	opParts := strings.SplitN(line, ":", 2)
	if len(opParts) != 2 {
		return fmt.Errorf("invalid operation line: %s", line)
	}
	opName := strings.TrimSpace(opParts[0])
	stats := strings.Fields(opParts[1])

	op, err := ParseNFSOperation(opName, stats)
	if err != nil {
		return err
	}
	p.currentMount.Operations[opName] = op
	return nil
}

// ParseMountstats parses /proc/self/mountstats and returns NFS mount information.
func ParseMountstats(path string) (map[string]*NFSMount, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	return ParseMountstatsReader(file)
}

// ParseMountstatsReader parses mountstats from an io.Reader for better testability
func ParseMountstatsReader(r io.Reader) (map[string]*NFSMount, error) {
	parser := &mountstatsParser{
		scanner: bufio.NewScanner(r),
		mounts:  make(map[string]*NFSMount),
	}

	if err := parser.parse(); err != nil {
		return nil, err
	}

	return parser.mounts, nil
}
			

// CalculateDelta computes the difference between two measurements.
func CalculateDelta(previousOp, currentOp *NFSOperation, durationSec float64) *DeltaStats {
	if previousOp == nil || currentOp == nil {
		return nil
	}

	deltaOps := currentOp.Ops - previousOp.Ops
	if deltaOps <= 0 {
		return &DeltaStats{
			Operation: currentOp.Name,
			DeltaOps:  0,
		}
	}

	delta := &DeltaStats{
		Operation:    currentOp.Name,
		DeltaOps:     deltaOps,
		DeltaSent:    currentOp.BytesSent - previousOp.BytesSent,
		DeltaRecv:    currentOp.BytesRecv - previousOp.BytesRecv,
		DeltaBytes:   (currentOp.BytesSent - previousOp.BytesSent) + (currentOp.BytesRecv - previousOp.BytesRecv),
		DeltaRTT:     currentOp.RTT - previousOp.RTT,
		DeltaExec:    currentOp.ExecuteTime - previousOp.ExecuteTime,
		DeltaQueue:   currentOp.QueueTime - previousOp.QueueTime,
		DeltaErrors:  currentOp.Errors - previousOp.Errors,
		DeltaRetrans: currentOp.Timeouts - previousOp.Timeouts,
		IOPS:         float64(deltaOps) / durationSec,
	}

	// Calculate averages
	delta.AvgRTT = float64(delta.DeltaRTT) / float64(deltaOps)
	delta.AvgExec = float64(delta.DeltaExec) / float64(deltaOps)
	delta.AvgQueue = float64(delta.DeltaQueue) / float64(deltaOps)
	delta.KBPerOp = float64(delta.DeltaBytes) / float64(deltaOps) / 1024
	delta.KBPerSec = float64(delta.DeltaBytes) / durationSec / 1024

	return delta
}

// DisplayStatsSimple shows stats in simple format with optional bandwidth.
func DisplayStatsSimple(mount *NFSMount, stats []*DeltaStats, showBandwidth bool, timestamp time.Time) {
	if len(stats) == 0 {
		return
	}

	// Print timestamp and mount point header
	fmt.Printf("\n%s mounted on %s:\n", mount.Device, timestamp.Format("01/02/2006 03:04:05 PM"))

	if showBandwidth {
		fmt.Printf("\n%-15s %10s %10s %10s %10s %10s\n",
			"Operation", "IOPS", "Avg RTT(ms)", "Avg Exec(ms)", "MB/s", "KB/op")
		fmt.Println(strings.Repeat("-", 75))
	} else {
		fmt.Printf("\n%-15s %10s %10s %10s\n",
			"Operation", "IOPS", "Avg RTT(ms)", "Avg Exec(ms)")
		fmt.Println(strings.Repeat("-", 52))
	}

	// Print stats for each operation
	for _, s := range stats {
		if s == nil || s.DeltaOps == 0 {
			continue
		}

		if showBandwidth {
			mbPerSec := s.KBPerSec / 1024
			fmt.Printf("% -15s %10.1f %10.3f %10.3f %10.3f %10.2f\n",
				s.Operation, s.IOPS, s.AvgRTT, s.AvgExec, mbPerSec, s.KBPerOp)
		} else {
			fmt.Printf("% -15s %10.1f %10.3f %10.3f\n",
				s.Operation, s.IOPS, s.AvgRTT, s.AvgExec)
		}
	}
}
