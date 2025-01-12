package main

import (
	"errors"
	"flag"
	"fmt"
	"httper/pkg/env"
	"httper/pkg/finalize"
	"httper/pkg/request"
	"log/slog"
	"net/http"
	"os"
	"path"
	"strings"
	"time"

	"golang.org/x/net/http2"
)

var (
	save = flag.Bool(
		"save",
		false,
		"save response to file",
	)
	envFile = flag.String(
		"env-file",
		"",
		"env file to be used to replace placeholders",
	)
	environment = flag.String(
		"env",
		"",
		"env to be used to replace placeholders",
	)
	verbose = flag.Bool(
		"v",
		false,
		"verbose output",
	)
)

func main() {
	flag.Parse()
	initLogger()

	if err := validateInput(); err != nil {
		slog.Error("validating input", "err", err)
		os.Exit(1)
	}

	if err := run(flag.Arg(0)); err != nil {
		slog.Error("running http", "err", err)
		os.Exit(1)
	}
}

func validateInput() error {
	input := flag.Arg(0)
	if input == "" {
		return errors.New("1st arg must be input file")
	}

	if _, err := os.Stat(input); err != nil {
		return fmt.Errorf("cannot stat file at %s", input)
	}

	if *envFile != "" {
		if _, err := os.Stat(*envFile); err != nil {
			return fmt.Errorf("cannot stat file at %s", *envFile)
		}
	}

	slog.Debug("input file", "name", input)

	return nil
}

func run(input string) error {
	contentRaw, err := os.ReadFile(input)
	if err != nil {
		return fmt.Errorf("cannot read file at %s: %w", input, err)
	}

	content := string(contentRaw)
	client := http.DefaultClient

	if *environment != "" {
		envMap := loadEnv(*envFile, *environment)
		if envMap != nil {
			content = envMap.Replace(content)
		}
	}

	httpRequests, err := request.Create(content, path.Dir(input))
	if err != nil {
		return fmt.Errorf("cannot create basic httpRequest: %w", err)
	}

	for _, httpRequest := range httpRequests {
		sendRequest(httpRequest, client)
	}

	return nil
}

func loadEnv(envFile, environment string) env.Environment {
	if envFile == "" {
		return nil
	}

	envs, err := env.Parse(envFile)
	if err != nil {
		slog.Error("parsing env file", "err", err)
		return nil
	}

	return envs[environment]
}

func sendRequest(httpRequest *http.Request, client *http.Client) {
	fmt.Println(httpRequest.URL)

	transport := http.DefaultTransport
	// todo prior knowledge
	if strings.HasPrefix(httpRequest.Proto, "HTTP/2") {
		transport = &http2.Transport{}
	}

	client.Transport = transport

	start := time.Now()
	response, err := client.Do(httpRequest)
	if err != nil {
		slog.Error("sending request", "err", err)
		return
	}

	defer func() {
		_ = response.Body.Close()
	}()

	finalize.Response(
		response,
		time.Since(start),
		*save,
		*verbose,
	)
}

func initLogger() {
	logger := slog.New(
		slog.NewTextHandler(
			os.Stdout, &slog.HandlerOptions{
				Level: slog.LevelInfo,
			},
		),
	)

	slog.SetDefault(logger)
}
