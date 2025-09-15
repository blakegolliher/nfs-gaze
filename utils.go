//go:build linux

package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"strconv"
	"strings"
	"time"
)

type Flags struct {
	MountPoint     string
	Operations     string
	Interval       time.Duration
	Count          int
	ShowAttr       bool
	ShowBandwidth  bool
	NfsiostatMode  bool
	ClearScreen    bool
	MountstatsPath string
}

// initFlags initializes and parses the command-line flags.
func initFlags() *Flags {
	flags := &Flags{}

	flag.StringVar(&flags.MountPoint, "m", "", "Mount point to monitor")
	flag.StringVar(&flags.Operations, "ops", "", "Comma-separated list of operations to monitor")
	flag.DurationVar(&flags.Interval, "i", 1*time.Second, "Update interval")
	flag.IntVar(&flags.Count, "c", 0, "Number of iterations (0 = infinite)")
	flag.BoolVar(&flags.ShowAttr, "attr", false, "Show attribute cache statistics")
	flag.BoolVar(&flags.ShowBandwidth, "bw", false, "Show bandwidth statistics")
	flag.BoolVar(&flags.NfsiostatMode, "nfsiostat", false, "Use nfsiostat output format")
	flag.BoolVar(&flags.ClearScreen, "clear", false, "Clear screen between iterations")
	flag.StringVar(&flags.MountstatsPath, "f", "/proc/self/mountstats", "Path to mountstats file")

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

	flag.Parse()

	args := flag.Args()
	if len(args) > 0 && flags.MountPoint == "" {
		flags.MountPoint = args[0]
	}
	if len(args) > 1 {
		if intervalSec, err := strconv.Atoi(args[1]); err == nil {
			flags.Interval = time.Duration(intervalSec) * time.Second
		}
	}
	if len(args) > 2 {
		if countVal, err := strconv.Atoi(args[2]); err == nil {
			flags.Count = countVal
		}
	}

	return flags
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
func getMountsToMonitor(mountPoint string, previousMounts map[string]*NFSMount) ([]string, error) {
	var monitorMounts []string
	if mountPoint != "" {
		if _, exists := previousMounts[mountPoint]; !exists {
			return nil, fmt.Errorf("mount point %s not found", mountPoint)
		}
		monitorMounts = append(monitorMounts, mountPoint)
	} else {
		for mp := range previousMounts {
			monitorMounts = append(monitorMounts, mp)
		}
		if len(monitorMounts) == 0 {
			return nil, fmt.Errorf("no NFS mounts found")
		}
	}
	return monitorMounts, nil
}

// printInitialSummary prints the initial summary of the monitored mounts.
func printInitialSummary(flags *Flags, monitorMounts []string, previousMounts map[string]*NFSMount, opsFilter map[string]bool) {
	if flags.NfsiostatMode {
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
				displayStatsNfsiostat(mount, stats, nil, flags.ShowAttr)
			}
		}
	} else {
		// Print header for simple mode
		fmt.Printf("Monitoring NFS mount: %s (%s)\n", monitorMounts[0], previousMounts[monitorMounts[0]].Device)
		fmt.Printf("Update interval: %v\n", flags.Interval)
		if flags.Operations != "" {
			fmt.Printf("Filtering operations: %s\n", flags.Operations)
		}
	}
}

// monitoringLoop is the main monitoring loop of the application.
func monitoringLoop(sigChan chan os.Signal, flags *Flags, monitorMounts []string, previousMounts map[string]*NFSMount, opsFilter map[string]bool) {
	// Monitoring loop
	iteration := 0
	ticker := time.NewTicker(flags.Interval)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			iteration++

			// Read current stats
			currentMounts, err := parseMountstats(flags.MountstatsPath)
			if err != nil {
				log.Printf("Error reading mountstats: %v", err)
				continue
			}

			timestamp := time.Now()
			duration := flags.Interval.Seconds()

			// Clear screen if requested (only for simple mode)
			if flags.ClearScreen && !flags.NfsiostatMode {
				fmt.Print("\033[H\033[2J")
				// Reprint header after clearing
				fmt.Printf("Monitoring NFS mount: %s (%s)\n", monitorMounts[0], currentMounts[monitorMounts[0]].Device)
				fmt.Printf("Update interval: %v | Time: %s\n", flags.Interval, timestamp.Format("15:04:05"))
				if flags.Operations != "" {
					fmt.Printf("Filtering operations: %s\n", flags.Operations)
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
				if flags.NfsiostatMode {
					// Always display in nfsiostat mode (even with no activity)
					displayStatsNfsiostat(currentMount, stats, previousMount, flags.ShowAttr)
				} else if len(stats) > 0 {
					// Simple mode - only show if there's activity
					displayStatsSimple(currentMount, stats, flags.ShowBandwidth, timestamp)
				}
			}

			// Update previous stats
			previousMounts = currentMounts

			// Check iteration count
			if flags.Count > 0 && iteration >= flags.Count {
				return
			}

		case <-sigChan:
			fmt.Println("\nCaught ^C... exiting")
			return
		}
	}
}
