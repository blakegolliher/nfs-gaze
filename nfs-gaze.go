package main

import (
	"bufio"
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
	"time"
)

// NFSOperation holds statistics for a single NFS operation
type NFSOperation struct {
	Name        string
	Ops         int64
	Ntrans      int64
	Timeouts    int64
	BytesSent   int64
	BytesRecv   int64
	QueueTime   int64 // in milliseconds
	RTT         int64 // in milliseconds
	ExecuteTime int64 // in milliseconds
	Errors      int64
}

// NFSEvents holds event statistics
type NFSEvents struct {
	InodeRevalidate  int64 // index 0
	DentryRevalidate int64 // index 1
	DataInvalidate   int64 // index 2
	AttrInvalidate   int64 // index 3
	VFSOpen          int64 // index 4
	VFSLookup        int64 // index 5
	VFSAccess        int64 // index 6
	VFSUpdatePage    int64 // index 7
	VFSReadPage      int64 // index 8
	VFSReadPages     int64 // index 9
	VFSWritePage     int64 // index 10
	VFSWritePages    int64 // index 11
	VFSGetdents      int64 // index 12
	VFSSetattr       int64 // index 13
	VFSFlush         int64 // index 14
	VFSFsync         int64 // index 15
	VFSLock          int64 // index 16
	VFSRelease       int64 // index 17
	CongestionWait   int64 // index 18
	SetattrTrunc     int64 // index 19
	ExtendWrite      int64 // index 20
	SillyRename      int64 // index 21
	ShortRead        int64 // index 22
	ShortWrite       int64 // index 23
	Delay            int64 // index 24
	PNFSRead         int64 // index 25
	PNFSWrite        int64 // index 26
}

// NFSMount represents a single NFS mount point
type NFSMount struct {
	Device     string
	MountPoint string
	Server     string
	Export     string
	Age        int64
	Operations map[string]*NFSOperation
	Events     *NFSEvents
	BytesRead  int64
	BytesWrite int64
}

// DeltaStats holds the difference between two measurements
type DeltaStats struct {
	Operation    string
	DeltaOps     int64
	DeltaBytes   int64
	DeltaSent    int64
	DeltaRecv    int64
	DeltaRTT     int64
	DeltaExec    int64
	DeltaQueue   int64
	DeltaErrors  int64
	DeltaRetrans int64
	AvgRTT       float64
	AvgExec      float64
	AvgQueue     float64
	KBPerOp      float64
	KBPerSec     float64
	IOPS         float64
}

// parseEvents parses the events line into an NFSEvents struct
func parseEvents(parts []string) *NFSEvents {
	events := &NFSEvents{}
	if len(parts) < 27 {
		return events
	}
	
	events.InodeRevalidate, _ = strconv.ParseInt(parts[0], 10, 64)
	events.DentryRevalidate, _ = strconv.ParseInt(parts[1], 10, 64)
	events.DataInvalidate, _ = strconv.ParseInt(parts[2], 10, 64)
	events.AttrInvalidate, _ = strconv.ParseInt(parts[3], 10, 64)
	events.VFSOpen, _ = strconv.ParseInt(parts[4], 10, 64)
	events.VFSLookup, _ = strconv.ParseInt(parts[5], 10, 64)
	events.VFSAccess, _ = strconv.ParseInt(parts[6], 10, 64)
	events.VFSUpdatePage, _ = strconv.ParseInt(parts[7], 10, 64)
	events.VFSReadPage, _ = strconv.ParseInt(parts[8], 10, 64)
	events.VFSReadPages, _ = strconv.ParseInt(parts[9], 10, 64)
	events.VFSWritePage, _ = strconv.ParseInt(parts[10], 10, 64)
	events.VFSWritePages, _ = strconv.ParseInt(parts[11], 10, 64)
	events.VFSGetdents, _ = strconv.ParseInt(parts[12], 10, 64)
	events.VFSSetattr, _ = strconv.ParseInt(parts[13], 10, 64)
	events.VFSFlush, _ = strconv.ParseInt(parts[14], 10, 64)
	events.VFSFsync, _ = strconv.ParseInt(parts[15], 10, 64)
	events.VFSLock, _ = strconv.ParseInt(parts[16], 10, 64)
	events.VFSRelease, _ = strconv.ParseInt(parts[17], 10, 64)
	events.CongestionWait, _ = strconv.ParseInt(parts[18], 10, 64)
	events.SetattrTrunc, _ = strconv.ParseInt(parts[19], 10, 64)
	events.ExtendWrite, _ = strconv.ParseInt(parts[20], 10, 64)
	events.SillyRename, _ = strconv.ParseInt(parts[21], 10, 64)
	events.ShortRead, _ = strconv.ParseInt(parts[22], 10, 64)
	events.ShortWrite, _ = strconv.ParseInt(parts[23], 10, 64)
	events.Delay, _ = strconv.ParseInt(parts[24], 10, 64)
	if len(parts) > 25 {
		events.PNFSRead, _ = strconv.ParseInt(parts[25], 10, 64)
	}
	if len(parts) > 26 {
		events.PNFSWrite, _ = strconv.ParseInt(parts[26], 10, 64)
	}
	
	return events
}

// parseMountstats parses /proc/self/mountstats and returns NFS mount information
func parseMountstats(path string) (map[string]*NFSMount, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	mounts := make(map[string]*NFSMount)
	scanner := bufio.NewScanner(file)
	
	var currentMount *NFSMount
	
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		
		// Parse device line
		if strings.HasPrefix(line, "device") && strings.Contains(line, "nfs") {
			parts := strings.Fields(line)
			if len(parts) >= 8 {
				serverExport := parts[1]
				mountPoint := parts[4]
				
				// Split server:export
				serverParts := strings.SplitN(serverExport, ":", 2)
				server := serverParts[0]
				export := "/"
				if len(serverParts) > 1 {
					export = serverParts[1]
				}
				
				currentMount = &NFSMount{
					Device:     serverExport,
					MountPoint: mountPoint,
					Server:     server,
					Export:     export,
					Operations: make(map[string]*NFSOperation),
					Events:     &NFSEvents{},
				}
				mounts[mountPoint] = currentMount
			}
		} else if currentMount != nil {
			// Parse stats for current mount
			if strings.HasPrefix(line, "age:") {
				parts := strings.Fields(line)
				if len(parts) >= 2 {
					currentMount.Age, _ = strconv.ParseInt(parts[1], 10, 64)
				}
			} else if strings.HasPrefix(line, "events:") {
				parts := strings.Fields(line)
				if len(parts) > 1 {
					currentMount.Events = parseEvents(parts[1:])
				}
			} else if strings.HasPrefix(line, "bytes:") {
				parts := strings.Fields(line)
				if len(parts) >= 5 {
					currentMount.BytesRead, _ = strconv.ParseInt(parts[1], 10, 64)
					currentMount.BytesWrite, _ = strconv.ParseInt(parts[5], 10, 64)
				}
			} else if strings.Contains(line, ":") && !strings.HasPrefix(line, "RPC") && 
				      !strings.HasPrefix(line, "xprt") && !strings.HasPrefix(line, "per-op") &&
				      !strings.HasPrefix(line, "opts") && !strings.HasPrefix(line, "caps") &&
				      !strings.HasPrefix(line, "sec") {
				// Parse operation statistics
				opParts := strings.SplitN(line, ":", 2)
				if len(opParts) == 2 {
					opName := strings.TrimSpace(opParts[0])
					stats := strings.Fields(opParts[1])
					
					if len(stats) >= 9 {
						op := &NFSOperation{
							Name: opName,
						}
						op.Ops, _ = strconv.ParseInt(stats[0], 10, 64)
						op.Ntrans, _ = strconv.ParseInt(stats[1], 10, 64)
						op.Timeouts, _ = strconv.ParseInt(stats[2], 10, 64)
						op.BytesSent, _ = strconv.ParseInt(stats[3], 10, 64)
						op.BytesRecv, _ = strconv.ParseInt(stats[4], 10, 64)
						op.QueueTime, _ = strconv.ParseInt(stats[5], 10, 64)
						op.RTT, _ = strconv.ParseInt(stats[6], 10, 64)
						op.ExecuteTime, _ = strconv.ParseInt(stats[7], 10, 64)
						if len(stats) > 8 {
							op.Errors, _ = strconv.ParseInt(stats[8], 10, 64)
						}
						
						currentMount.Operations[opName] = op
					}
				}
			}
		}
	}
	
	return mounts, scanner.Err()
}

// calculateDelta computes the difference between two measurements
func calculateDelta(old, new *NFSOperation, duration float64) *DeltaStats {
	if old == nil || new == nil {
		return nil
	}
	
	deltaOps := new.Ops - old.Ops
	if deltaOps <= 0 {
		return &DeltaStats{
			Operation: new.Name,
			DeltaOps:  0,
		}
	}
	
	delta := &DeltaStats{
		Operation:    new.Name,
		DeltaOps:     deltaOps,
		DeltaSent:    new.BytesSent - old.BytesSent,
		DeltaRecv:    new.BytesRecv - old.BytesRecv,
		DeltaBytes:   (new.BytesSent - old.BytesSent) + (new.BytesRecv - old.BytesRecv),
		DeltaRTT:     new.RTT - old.RTT,
		DeltaExec:    new.ExecuteTime - old.ExecuteTime,
		DeltaQueue:   new.QueueTime - old.QueueTime,
		DeltaErrors:  new.Errors - old.Errors,
		DeltaRetrans: new.Timeouts - old.Timeouts,
		IOPS:         float64(deltaOps) / duration,
	}
	
	// Calculate averages
	delta.AvgRTT = float64(delta.DeltaRTT) / float64(deltaOps)
	delta.AvgExec = float64(delta.DeltaExec) / float64(deltaOps)
	delta.AvgQueue = float64(delta.DeltaQueue) / float64(deltaOps)
	delta.KBPerOp = float64(delta.DeltaBytes) / float64(deltaOps) / 1024
	delta.KBPerSec = float64(delta.DeltaBytes) / duration / 1024
	
	return delta
}

// displayStatsNfsiostat shows stats in nfsiostat format
func displayStatsNfsiostat(mount *NFSMount, stats []*DeltaStats, previousMount *NFSMount, showAttr bool) {
	// Calculate total ops/s
	totalOps := float64(0)
	for _, s := range stats {
		if s != nil {
			totalOps += s.IOPS
		}
	}
	
	// Print mount header and summary
	fmt.Printf("\n%s mounted on %s:\n\n", mount.Device, mount.MountPoint)
	fmt.Printf("%16s %16s\n", "ops/s", "rpc bklog")
	fmt.Printf("%16.3f %16.3f\n\n", totalOps, 0.000)
	
	// Print per-operation statistics
	for _, s := range stats {
		if s == nil || s.DeltaOps == 0 {
			continue
		}
		
		opName := strings.ToLower(s.Operation)
		fmt.Printf("%s:", opName)
		
		// Calculate error and retrans percentages
		errorPct := float64(0)
		if s.DeltaOps > 0 {
			errorPct = float64(s.DeltaErrors) / float64(s.DeltaOps) * 100
		}
		retransPct := float64(0)
		if s.DeltaOps > 0 {
			retransPct = float64(s.DeltaRetrans) / float64(s.DeltaOps) * 100
		}
		
		// Print header for this operation
		fmt.Printf("%16s %16s %16s %16s %16s %16s %16s %16s\n",
			"ops/s", "kB/s", "kB/op", "retrans", "avg RTT (ms)", "avg exe (ms)", "avg queue (ms)", "errors")
		
		// Print values
		fmt.Printf("%26.3f %16.3f %16.3f %8d (%.1f%%) %16.3f %16.3f %16.3f %8d (%.1f%%)\n",
			s.IOPS, s.KBPerSec, s.KBPerOp, s.DeltaRetrans, retransPct,
			s.AvgRTT, s.AvgExec, s.AvgQueue, s.DeltaErrors, errorPct)
	}
	
	// Print attribute cache statistics if requested
	if showAttr && previousMount != nil {
		fmt.Printf("\n")
		vfsOpens := mount.Events.VFSOpen - previousMount.Events.VFSOpen
		inodeRevals := mount.Events.InodeRevalidate - previousMount.Events.InodeRevalidate
		pageInvals := mount.Events.DataInvalidate - previousMount.Events.DataInvalidate
		attrInvals := mount.Events.AttrInvalidate - previousMount.Events.AttrInvalidate
		
		fmt.Printf("%d VFS opens\n", vfsOpens)
		fmt.Printf("%d inoderevalidates (forced GETATTRs)\n", inodeRevals)
		fmt.Printf("%d page cache invalidations\n", pageInvals)
		fmt.Printf("%d attribute cache invalidations\n", attrInvals)
	}
}

// displayStatsSimple shows stats in simple format with optional bandwidth
func displayStatsSimple(mount *NFSMount, stats []*DeltaStats, showBandwidth bool, timestamp time.Time) {
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
			fmt.Printf("%-15s %10.1f %10.3f %10.3f %10.3f %10.2f\n",
				s.Operation, s.IOPS, s.AvgRTT, s.AvgExec, mbPerSec, s.KBPerOp)
		} else {
			fmt.Printf("%-15s %10.1f %10.3f %10.3f\n",
				s.Operation, s.IOPS, s.AvgRTT, s.AvgExec)
		}
	}
}

func main() {
	// Command-line flags
	var (
		mountPoint     = flag.String("m", "", "Mount point to monitor")
		operations     = flag.String("ops", "", "Comma-separated list of operations to monitor")
		interval       = flag.Duration("i", 1*time.Second, "Update interval")
		count          = flag.Int("c", 0, "Number of iterations (0 = infinite)")
		showAttr       = flag.Bool("attr", false, "Show attribute cache statistics")
		showBandwidth  = flag.Bool("bw", false, "Show bandwidth statistics")
		nfsiostatMode  = flag.Bool("nfsiostat", false, "Use nfsiostat output format")
		clearScreen    = flag.Bool("clear", false, "Clear screen between iterations")
		mountstatsPath = flag.String("f", "/proc/self/mountstats", "Path to mountstats file")
	)
	
	flag.Usage = func() {
		fmt.Fprintf(os.Stderr, "NFS I/O Statistics Monitor\n\n")
		fmt.Fprintf(os.Stderr, "Usage: %s [options] [mount_point] [interval] [count]\n\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "Options:\n")
		flag.PrintDefaults()
		fmt.Fprintf(os.Stderr, "\nExamples:\n")
		fmt.Fprintf(os.Stderr, "  # Monitor in nfsiostat format\n")
		fmt.Fprintf(os.Stderr, "  %s --nfsiostat /mnt/nfs --attr\n\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  # Monitor specific operations with bandwidth\n")
		fmt.Fprintf(os.Stderr, "  %s -m /mnt/nfs -ops READ,WRITE -bw\n\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  # Clear screen between iterations\n")
		fmt.Fprintf(os.Stderr, "  %s -m /mnt/nfs --clear\n\n", os.Args[0])
	}
	
	// Parse positional arguments (for compatibility)
	nonFlagArgs := []string{}
	for i := 1; i < len(os.Args); i++ {
		if strings.HasPrefix(os.Args[i], "-") {
			break
		}
		nonFlagArgs = append(nonFlagArgs, os.Args[i])
	}
	
	if len(nonFlagArgs) > 0 && *mountPoint == "" {
		*mountPoint = nonFlagArgs[0]
	}
	if len(nonFlagArgs) > 1 {
		if intervalSec, err := strconv.Atoi(nonFlagArgs[1]); err == nil {
			*interval = time.Duration(intervalSec) * time.Second
		}
	}
	if len(nonFlagArgs) > 2 {
		if countVal, err := strconv.Atoi(nonFlagArgs[2]); err == nil {
			*count = countVal
		}
	}
	
	flag.Parse()
	
	// Parse operations filter
	var opsFilter map[string]bool
	if *operations != "" {
		opsFilter = make(map[string]bool)
		for _, op := range strings.Split(*operations, ",") {
			opsFilter[strings.TrimSpace(op)] = true
		}
	}
	
	// Setup signal handling
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)
	
	// Initial read
	previousMounts, err := parseMountstats(*mountstatsPath)
	if err != nil {
		log.Fatal("Error reading mountstats: ", err)
	}
	
	// Determine which mounts to monitor
	var monitorMounts []string
	if *mountPoint != "" {
		if _, exists := previousMounts[*mountPoint]; !exists {
			log.Fatalf("Mount point %s not found", *mountPoint)
		}
		monitorMounts = append(monitorMounts, *mountPoint)
	} else {
		for mp := range previousMounts {
			monitorMounts = append(monitorMounts, mp)
		}
		if len(monitorMounts) == 0 {
			fmt.Fprintf(os.Stderr, "No NFS mounts found\n")
			os.Exit(1)
		}
	}
	
	// First iteration - show cumulative stats
	if *nfsiostatMode {
		for _, mp := range monitorMounts {
			mount := previousMounts[mp]
			if mount == nil {
				continue
			}
			
			var stats []*DeltaStats
			mountAgeSec := float64(mount.Age)
			
			for _, op := range mount.Operations {
				if op.Ops > 0 {
					// Apply filter if specified
					if opsFilter != nil && !opsFilter[op.Name] {
						continue
					}
					
					delta := &DeltaStats{
						Operation:    op.Name,
						DeltaOps:     op.Ops,
						DeltaSent:    op.BytesSent,
						DeltaRecv:    op.BytesRecv,
						DeltaBytes:   op.BytesSent + op.BytesRecv,
						DeltaRTT:     op.RTT,
						DeltaExec:    op.ExecuteTime,
						DeltaQueue:   op.QueueTime,
						DeltaErrors:  op.Errors,
						DeltaRetrans: op.Timeouts,
						IOPS:         float64(op.Ops) / mountAgeSec,
						AvgRTT:       float64(op.RTT) / float64(op.Ops),
						AvgExec:      float64(op.ExecuteTime) / float64(op.Ops),
						AvgQueue:     float64(op.QueueTime) / float64(op.Ops),
						KBPerOp:      float64(op.BytesSent+op.BytesRecv) / float64(op.Ops) / 1024,
						KBPerSec:     float64(op.BytesSent+op.BytesRecv) / mountAgeSec / 1024,
					}
					stats = append(stats, delta)
				}
			}
			
			if len(stats) > 0 {
				displayStatsNfsiostat(mount, stats, nil, *showAttr)
			}
		}
	} else {
		// Print header for simple mode
		fmt.Printf("Monitoring NFS mount: %s (%s)\n", monitorMounts[0], previousMounts[monitorMounts[0]].Device)
		fmt.Printf("Update interval: %v\n", *interval)
		if *operations != "" {
			fmt.Printf("Filtering operations: %s\n", *operations)
		}
	}
	
	// Monitoring loop
	iteration := 0
	ticker := time.NewTicker(*interval)
	defer ticker.Stop()
	
	for {
		select {
		case <-ticker.C:
			iteration++
			
			// Read current stats
			currentMounts, err := parseMountstats(*mountstatsPath)
			if err != nil {
				log.Printf("Error reading mountstats: %v", err)
				continue
			}
			
			timestamp := time.Now()
			duration := interval.Seconds()
			
			// Clear screen if requested (only for simple mode)
			if *clearScreen && !*nfsiostatMode {
				fmt.Print("\033[H\033[2J")
				// Reprint header after clearing
				fmt.Printf("Monitoring NFS mount: %s (%s)\n", monitorMounts[0], currentMounts[monitorMounts[0]].Device)
				fmt.Printf("Update interval: %v | Time: %s\n", *interval, timestamp.Format("15:04:05"))
				if *operations != "" {
					fmt.Printf("Filtering operations: %s\n", *operations)
				}
			}
			
			// Process each monitored mount
			for _, mp := range monitorMounts {
				currentMount, exists := currentMounts[mp]
				if !exists {
					continue
				}
				
				previousMount := previousMounts[mp]
				if previousMount == nil {
					continue
				}
				
				// Calculate deltas
				var stats []*DeltaStats
				
				for opName, currentOp := range currentMount.Operations {
					// Apply filter if specified
					if opsFilter != nil && !opsFilter[opName] {
						continue
					}
					
					previousOp := previousMount.Operations[opName]
					if previousOp != nil {
						delta := calculateDelta(previousOp, currentOp, duration)
						if delta != nil && delta.DeltaOps > 0 {
							stats = append(stats, delta)
						}
					}
				}
				
				// Display results
				if *nfsiostatMode {
					// Always display in nfsiostat mode (even with no activity)
					displayStatsNfsiostat(currentMount, stats, previousMount, *showAttr)
				} else if len(stats) > 0 {
					// Simple mode - only show if there's activity
					displayStatsSimple(currentMount, stats, *showBandwidth, timestamp)
				}
			}
			
			// Update previous stats
			previousMounts = currentMounts
			
			// Check iteration count
			if *count > 0 && iteration >= *count {
				return
			}
			
		case <-sigChan:
			fmt.Println("\nCaught ^C... exiting")
			return
		}
	}
}
