//go:build linux

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



// parseEvents parses the events line into an NFSEvents struct
func parseEvents(parts []string) (*NFSEvents, error) {
	events := &NFSEvents{}
	if len(parts) < 27 {
		return events, fmt.Errorf("invalid number of parts for events: %d", len(parts))
	}

	var err error
	events.InodeRevalidate, err = strconv.ParseInt(parts[0], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing InodeRevalidate: %w", err)
	}
	events.DentryRevalidate, err = strconv.ParseInt(parts[1], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing DentryRevalidate: %w", err)
	}
	events.DataInvalidate, err = strconv.ParseInt(parts[2], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing DataInvalidate: %w", err)
	}
	events.AttrInvalidate, err = strconv.ParseInt(parts[3], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing AttrInvalidate: %w", err)
	}
	events.VFSOpen, err = strconv.ParseInt(parts[4], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSOpen: %w", err)
	}
	events.VFSLookup, err = strconv.ParseInt(parts[5], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSLookup: %w", err)
	}
	events.VFSAccess, err = strconv.ParseInt(parts[6], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSAccess: %w", err)
	}
	events.VFSUpdatePage, err = strconv.ParseInt(parts[7], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSUpdatePage: %w", err)
	}
	events.VFSReadPage, err = strconv.ParseInt(parts[8], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSReadPage: %w", err)
	}
	events.VFSReadPages, err = strconv.ParseInt(parts[9], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSReadPages: %w", err)
	}
	events.VFSWritePage, err = strconv.ParseInt(parts[10], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSWritePage: %w", err)
	}
	events.VFSWritePages, err = strconv.ParseInt(parts[11], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSWritePages: %w", err)
	}
	events.VFSGetdents, err = strconv.ParseInt(parts[12], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSGetdents: %w", err)
	}
	events.VFSSetattr, err = strconv.ParseInt(parts[13], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSSetattr: %w", err)
	}
	events.VFSFlush, err = strconv.ParseInt(parts[14], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSFlush: %w", err)
	}
	events.VFSFsync, err = strconv.ParseInt(parts[15], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSFsync: %w", err)
	}
	events.VFSLock, err = strconv.ParseInt(parts[16], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSLock: %w", err)
	}
	events.VFSRelease, err = strconv.ParseInt(parts[17], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing VFSRelease: %w", err)
	}
	events.CongestionWait, err = strconv.ParseInt(parts[18], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing CongestionWait: %w", err)
	}
	events.SetattrTrunc, err = strconv.ParseInt(parts[19], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing SetattrTrunc: %w", err)
	}
	events.ExtendWrite, err = strconv.ParseInt(parts[20], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing ExtendWrite: %w", err)
	}
	events.SillyRename, err = strconv.ParseInt(parts[21], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing SillyRename: %w", err)
	}
	events.ShortRead, err = strconv.ParseInt(parts[22], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing ShortRead: %w", err)
	}
	events.ShortWrite, err = strconv.ParseInt(parts[23], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing ShortWrite: %w", err)
	}
	events.Delay, err = strconv.ParseInt(parts[24], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("error parsing Delay: %w", err)
	}
	if len(parts) > 25 {
		events.PNFSRead, err = strconv.ParseInt(parts[25], 10, 64)
		if err != nil {
			return nil, fmt.Errorf("error parsing PNFSRead: %w", err)
		}
	}
	if len(parts) > 26 {
		events.PNFSWrite, err = strconv.ParseInt(parts[26], 10, 64)
		if err != nil {
			return nil, fmt.Errorf("error parsing PNFSWrite: %w", err)
		}
	}

	return events, nil
}

// parseMountstats parses /proc/self/mountstats and returns NFS mount information.
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
					var err error
					currentMount.Age, err = strconv.ParseInt(parts[1], 10, 64)
					if err != nil {
						log.Printf("error parsing age for mount %s: %v", currentMount.MountPoint, err)
					}
				}
			} else if strings.HasPrefix(line, "events:") {
				parts := strings.Fields(line)
				if len(parts) > 1 {
					var err error
					currentMount.Events, err = parseEvents(parts[1:])
					if err != nil {
						log.Printf("error parsing events for mount %s: %v", currentMount.MountPoint, err)
					}
				}
			} else if strings.HasPrefix(line, "bytes:") {
				parts := strings.Fields(line)
				if len(parts) >= 5 {
					var err error
					currentMount.BytesRead, err = strconv.ParseInt(parts[1], 10, 64)
					if err != nil {
						log.Printf("error parsing bytes read for mount %s: %v", currentMount.MountPoint, err)
					}
					currentMount.BytesWrite, err = strconv.ParseInt(parts[5], 10, 64)
					if err != nil {
						log.Printf("error parsing bytes write for mount %s: %v", currentMount.MountPoint, err)
					}
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
						var err error
						op := &NFSOperation{
							Name: opName,
						}
						op.Ops, err = strconv.ParseInt(stats[0], 10, 64)
						if err != nil {
							log.Printf("error parsing ops for %s on mount %s: %v", opName, currentMount.MountPoint, err)
						}
						op.Ntrans, err = strconv.ParseInt(stats[1], 10, 64)
						if err != nil {
							log.Printf("error parsing ntrans for %s on mount %s: %v", opName, currentMount.MountPoint, err)
						}
						op.Timeouts, err = strconv.ParseInt(stats[2], 10, 64)
						if err != nil {
							log.Printf("error parsing timeouts for %s on mount %s: %v", opName, currentMount.MountPoint, err)
						}
						op.BytesSent, err = strconv.ParseInt(stats[3], 10, 64)
						if err != nil {
							log.Printf("error parsing bytes sent for %s on mount %s: %v", opName, currentMount.MountPoint, err)
						}
						op.BytesRecv, err = strconv.ParseInt(stats[4], 10, 64)
						if err != nil {
							log.Printf("error parsing bytes recv for %s on mount %s: %v", opName, currentMount.MountPoint, err)
						}
						op.QueueTime, err = strconv.ParseInt(stats[5], 10, 64)
						if err != nil {
							log.Printf("error parsing queue time for %s on mount %s: %v", opName, currentMount.MountPoint, err)
						}
						op.RTT, err = strconv.ParseInt(stats[6], 10, 64)
						if err != nil {
							log.Printf("error parsing rtt for %s on mount %s: %v", opName, currentMount.MountPoint, err)
						}
						op.ExecuteTime, err = strconv.ParseInt(stats[7], 10, 64)
						if err != nil {
							log.Printf("error parsing execute time for %s on mount %s: %v", opName, currentMount.MountPoint, err)
						}
						if len(stats) > 8 {
							op.Errors, err = strconv.ParseInt(stats[8], 10, 64)
							if err != nil {
								log.Printf("error parsing errors for %s on mount %s: %v", opName, currentMount.MountPoint, err)
							}
						}
						
						currentMount.Operations[opName] = op
					}
				}
			}
		}
	}
	
	return mounts, scanner.Err()
}

// calculateDelta computes the difference between two measurements.
func calculateDelta(previousOp, currentOp *NFSOperation, durationSec float64) *DeltaStats {
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

// displayStatsNfsiostat shows stats in nfsiostat format. 
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

// displayStatsSimple shows stats in simple format with optional bandwidth. 
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





// parseOperationsFilter parses the comma-separated list of operations to monitor.
func parseOperationsFilter(operations string) map[string]bool {
	var opsFilter map[string]bool
	if operations != "" {
		opsFilter = make(map[string]bool)
		for _, op := range strings.Split(operations, ",") {
			opsFilter[strings.TrimSpace(op)] = true
		}
	}
	return opsFilter
}

// getMountsToMonitor determines which mounts to monitor based on user input.
func getMountsToMonitor(mountPoint string, previousMounts map[string]*NFSMount) []string {
	var monitorMounts []string
	if mountPoint != "" {
		if _, exists := previousMounts[mountPoint]; !exists {
			log.Fatalf("Mount point %s not found", mountPoint)
		}
		monitorMounts = append(monitorMounts, mountPoint)
	} else {
		for mp := range previousMounts {
			monitorMounts = append(monitorMounts, mp)
		}
		if len(monitorMounts) == 0 {
			fmt.Fprintf(os.Stderr, "No NFS mounts found\n")
			os.Exit(1)
		}
	}
	return monitorMounts
}

// printInitialSummary prints the initial summary of the monitored mounts.
func printInitialSummary(nfsiostatMode bool, monitorMounts []string, previousMounts map[string]*NFSMount, opsFilter map[string]bool, showAttr bool, operations string, interval time.Duration) {
	if nfsiostatMode {
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
				displayStatsNfsiostat(mount, stats, nil, showAttr)
			}
		}
	} else {
		// Print header for simple mode
		fmt.Printf("Monitoring NFS mount: %s (%s)\n", monitorMounts[0], previousMounts[monitorMounts[0]].Device)
		fmt.Printf("Update interval: %v\n", interval)
		if operations != "" {
			fmt.Printf("Filtering operations: %s\n", operations)
		}
	}
}

// monitoringLoop is the main monitoring loop of the application.
func monitoringLoop(sigChan chan os.Signal, interval time.Duration, count int, mountstatsPath string, clearScreen bool, nfsiostatMode bool, monitorMounts []string, previousMounts map[string]*NFSMount, opsFilter map[string]bool, showAttr bool, showBandwidth bool, operations string) {
	// Monitoring loop
	iteration := 0
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			iteration++

			// Read current stats
			currentMounts, err := parseMountstats(mountstatsPath)
			if err != nil {
				log.Printf("Error reading mountstats: %v", err)
				continue
			}

			timestamp := time.Now()
			duration := interval.Seconds()

			// Clear screen if requested (only for simple mode)
			if clearScreen && !nfsiostatMode {
				fmt.Print("\033[H\033[2J")
				// Reprint header after clearing
				fmt.Printf("Monitoring NFS mount: %s (%s)\n", monitorMounts[0], currentMounts[monitorMounts[0]].Device)
				fmt.Printf("Update interval: %v | Time: %s\n", interval, timestamp.Format("15:04:05"))
				if operations != "" {
					fmt.Printf("Filtering operations: %s\n", operations)
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
				if nfsiostatMode {
					// Always display in nfsiostat mode (even with no activity)
					displayStatsNfsiostat(currentMount, stats, previousMount, showAttr)
				} else if len(stats) > 0 {
					// Simple mode - only show if there's activity
					displayStatsSimple(currentMount, stats, showBandwidth, timestamp)
				}
			}

			// Update previous stats
			previousMounts = currentMounts

			// Check iteration count
			if count > 0 && iteration >= count {
				return
			}

		case <-sigChan:
			fmt.Println("\nCaught ^C... exiting")
			return
		}
	}
}

