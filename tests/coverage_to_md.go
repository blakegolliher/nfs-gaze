//go:build ignore

package main

import (
	"fmt"
	"os"
	"os/exec"
	"sort"
	"strconv"
	"strings"
	"time"
)

type Coverage struct {
	File     string
	Function string
	Coverage float64
}

type FileCoverage struct {
	Name      string
	Functions []Coverage
	Total     float64
}

func main() {
	// Determine coverage file location
	coverageFile := "tests/coverage.out"
	if _, err := os.Stat(coverageFile); os.IsNotExist(err) {
		// Try current directory if we're running from tests/
		coverageFile = "coverage.out"
		if _, err := os.Stat(coverageFile); os.IsNotExist(err) {
			fmt.Fprintf(os.Stderr, "Coverage file not found. Run 'make coverage' first.\n")
			os.Exit(1)
		}
	}

	// Get coverage data
	cmd := exec.Command("go", "tool", "cover", "-func="+coverageFile)
	output, err := cmd.Output()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error getting coverage: %v\n", err)
		os.Exit(1)
	}

	// Parse coverage data
	lines := strings.Split(string(output), "\n")
	fileMap := make(map[string]*FileCoverage)
	var totalCoverage float64

	for _, line := range lines {
		if line == "" {
			continue
		}

		parts := strings.Fields(line)
		if len(parts) < 3 {
			continue
		}

		// Parse the line
		fileFuncParts := strings.Split(parts[0], ":")
		if len(fileFuncParts) < 2 {
			continue
		}

		fileName := fileFuncParts[0]
		funcName := parts[1]
		coverageStr := parts[2]

		// Handle total line
		if strings.HasPrefix(line, "total:") {
			coverageStr = parts[len(parts)-1]
			coverageStr = strings.TrimSuffix(coverageStr, "%")
			totalCoverage, _ = strconv.ParseFloat(coverageStr, 64)
			continue
		}

		// Parse coverage percentage
		coverageStr = strings.TrimSuffix(coverageStr, "%")
		coverage, err := strconv.ParseFloat(coverageStr, 64)
		if err != nil {
			continue
		}

		// Group by file
		if _, exists := fileMap[fileName]; !exists {
			fileMap[fileName] = &FileCoverage{
				Name:      fileName,
				Functions: []Coverage{},
			}
		}

		fileMap[fileName].Functions = append(fileMap[fileName].Functions, Coverage{
			File:     fileName,
			Function: funcName,
			Coverage: coverage,
		})
	}

	// Calculate file totals
	for _, file := range fileMap {
		if len(file.Functions) > 0 {
			sum := 0.0
			for _, fn := range file.Functions {
				sum += fn.Coverage
			}
			file.Total = sum / float64(len(file.Functions))
		}
	}

	// Sort files by name
	var files []*FileCoverage
	for _, file := range fileMap {
		files = append(files, file)
	}
	sort.Slice(files, func(i, j int) bool {
		return files[i].Name < files[j].Name
	})

	// Generate markdown report
	report := &strings.Builder{}

	// Header
	fmt.Fprintf(report, "# Test Coverage Report\n\n")
	fmt.Fprintf(report, "Generated: %s\n\n", time.Now().Format("2006-01-02 15:04:05"))

	// Overall coverage
	fmt.Fprintf(report, "## Overall Coverage: %.1f%%\n\n", totalCoverage)

	// Coverage badge
	badgeColor := "red"
	if totalCoverage >= 80 {
		badgeColor = "brightgreen"
	} else if totalCoverage >= 60 {
		badgeColor = "yellow"
	} else if totalCoverage >= 40 {
		badgeColor = "orange"
	}
	fmt.Fprintf(report, "![Coverage](https://img.shields.io/badge/coverage-%.1f%%-%%25-%s)\n\n", totalCoverage, badgeColor)

	// Summary table
	fmt.Fprintf(report, "## Summary by File\n\n")
	fmt.Fprintf(report, "| File | Coverage | Status |\n")
	fmt.Fprintf(report, "|------|----------|--------|\n")

	for _, file := range files {
		status := "❌"
		if file.Total >= 80 {
			status = "✅"
		} else if file.Total >= 60 {
			status = "⚠️"
		}

		// Clean up file name
		fileName := strings.Replace(file.Name, "nfs-gaze/", "", 1)
		fmt.Fprintf(report, "| %s | %.1f%% | %s |\n", fileName, file.Total, status)
	}

	// Detailed breakdown
	fmt.Fprintf(report, "\n## Detailed Coverage\n\n")

	for _, file := range files {
		fileName := strings.Replace(file.Name, "nfs-gaze/", "", 1)
		fmt.Fprintf(report, "### %s (%.1f%%)\n\n", fileName, file.Total)

		if len(file.Functions) > 0 {
			fmt.Fprintf(report, "<details>\n")
			fmt.Fprintf(report, "<summary>Function Coverage</summary>\n\n")
			fmt.Fprintf(report, "| Function | Coverage |\n")
			fmt.Fprintf(report, "|----------|----------|\n")

			// Sort functions by coverage (lowest first)
			sort.Slice(file.Functions, func(i, j int) bool {
				return file.Functions[i].Coverage < file.Functions[j].Coverage
			})

			for _, fn := range file.Functions {
				indicator := "🔴"
				if fn.Coverage >= 80 {
					indicator = "🟢"
				} else if fn.Coverage >= 60 {
					indicator = "🟡"
				} else if fn.Coverage > 0 {
					indicator = "🟠"
				}
				fmt.Fprintf(report, "| %s %s | %.1f%% |\n", indicator, fn.Function, fn.Coverage)
			}

			fmt.Fprintf(report, "\n</details>\n\n")
		}
	}

	// Coverage goals
	fmt.Fprintf(report, "## Coverage Targets\n\n")
	fmt.Fprintf(report, "- 🟢 **Good**: >= 80%%\n")
	fmt.Fprintf(report, "- 🟡 **Acceptable**: >= 60%%\n")
	fmt.Fprintf(report, "- 🟠 **Needs Improvement**: > 0%%\n")
	fmt.Fprintf(report, "- 🔴 **Not Covered**: 0%%\n\n")

	// Recommendations
	fmt.Fprintf(report, "## Recommendations\n\n")

	var uncovered []string
	var lowCoverage []string

	for _, file := range files {
		fileName := strings.Replace(file.Name, "nfs-gaze/", "", 1)
		for _, fn := range file.Functions {
			if fn.Coverage == 0 {
				uncovered = append(uncovered, fmt.Sprintf("`%s` in %s", fn.Function, fileName))
			} else if fn.Coverage < 60 {
				lowCoverage = append(lowCoverage, fmt.Sprintf("`%s` in %s (%.1f%%)", fn.Function, fileName, fn.Coverage))
			}
		}
	}

	if len(uncovered) > 0 {
		fmt.Fprintf(report, "### Functions with No Coverage\n\n")
		for _, fn := range uncovered {
			fmt.Fprintf(report, "- %s\n", fn)
		}
		fmt.Fprintf(report, "\n")
	}

	if len(lowCoverage) > 0 {
		fmt.Fprintf(report, "### Functions with Low Coverage (<60%%)\n\n")
		for _, fn := range lowCoverage {
			fmt.Fprintf(report, "- %s\n", fn)
		}
		fmt.Fprintf(report, "\n")
	}

	// How to run tests
	fmt.Fprintf(report, "## Running Tests\n\n")
	fmt.Fprintf(report, "```bash\n")
	fmt.Fprintf(report, "# Run all tests\n")
	fmt.Fprintf(report, "go test ./...\n\n")
	fmt.Fprintf(report, "# Run with coverage (from project root)\n")
	fmt.Fprintf(report, "go test -coverprofile=tests/coverage.out ./...\n\n")
	fmt.Fprintf(report, "# Generate this report using Makefile\n")
	fmt.Fprintf(report, "make coverage\n\n")
	fmt.Fprintf(report, "# Or manually\n")
	fmt.Fprintf(report, "go run tests/coverage_to_md.go > tests/COVERAGE.md\n")
	fmt.Fprintf(report, "```\n")

	// Write to stdout (can be redirected to file)
	fmt.Print(report.String())
}