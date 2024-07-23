package main

import (
	"flag"
	"fmt"
	"httper/pkg/request"
	"io"
	"log/slog"
	"net/http"
	"net/http/httputil"
	"os"
	"path"
	"text/tabwriter"
	"time"
)

var (
	client = &http.Client{}

	debug = flag.Bool(
		"debug",
		false,
		"debug mode, more verbose output",
	)
)

func main() {
	flag.Parse()
	initLogger()

	input := flag.Arg(0)
	if input == "" {
		slog.Error("1st arg must be input file")
		os.Exit(1)
	}

	slog.Debug("input file", "name", input)

	if err := run(input); err != nil {
		slog.Error("running http", "err", err)
		os.Exit(1)
	}
}

func run(input string) error {
	if _, err := os.Stat(input); err != nil {
		return fmt.Errorf("cannot stat file at %s: %w", input, err)
	}

	contentRaw, err := os.ReadFile(input)
	if err != nil {
		return fmt.Errorf("cannot read file at %s: %w", input, err)
	}

	content := string(contentRaw)
	httpRequests, err := request.Create(content, path.Dir(input))
	if err != nil {
		return fmt.Errorf("cannot create basic httpRequest: %w", err)
	}

	for _, httpRequest := range httpRequests {
		sendRequest(httpRequest)
	}

	return nil
}

func sendRequest(httpRequest *http.Request) {
	if *debug {
		if dump, err := httputil.DumpRequest(httpRequest, true); err == nil {
			slog.Debug("http request", "dump", string(dump))
		} else {
			slog.Info("request", "request", httpRequest)
		}
	}

	start := time.Now()
	response, err := client.Do(httpRequest)
	if err != nil {
		slog.Error("sending request", "err", err)
	}

	printResult(response, time.Since(start))
}

func printResult(response *http.Response, duration time.Duration) {
	responseBody, err := io.ReadAll(response.Body)
	if err != nil {
		slog.Error("reading response body", "err", err)
	}

	w := tabwriter.NewWriter(os.Stdout, 20, 20, 1, ' ', tabwriter.Debug)

	_, _ = fmt.Fprintln(w)
	_, _ = fmt.Fprintf(w, "Status\t%d\n", response.StatusCode)
	_, _ = fmt.Fprintf(w, "Duration\t%s\n", duration)
	_, _ = fmt.Fprintf(w, "Content-Length\t%d\n", len(responseBody))

	if *debug {
		_, _ = fmt.Fprintf(w, "Response:\n%s\n", string(responseBody))
	}

	if err = w.Flush(); err != nil {
		slog.Error("flushing tabwriter", "err", err)
	}
}

func initLogger() {
	level := slog.LevelInfo
	if *debug {
		level = slog.LevelDebug
	}

	logger := slog.New(
		slog.NewTextHandler(
			os.Stdout, &slog.HandlerOptions{
				Level: level,
			},
		),
	)
	slog.SetDefault(logger)
}
