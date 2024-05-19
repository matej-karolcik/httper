package main

import (
	"errors"
	"flag"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"net/url"
	"os"
	"strings"
)

func main() {
	flag.Parse()

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelDebug,
	}))
	slog.SetDefault(logger)

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

	head, body, _ := strings.Cut(content, "\n\n")
	essentials, headersRaw, _ := strings.Cut(head, "\n")

	headers := getHeaders(headersRaw)
	bodyReader, err := getBody(headers.Get("Content-Type"), body)
	if err != nil {
		return fmt.Errorf("getting request body: %w", err)
	}

	request, err := createRequest(essentials, headers, bodyReader)
	if err != nil {
		return fmt.Errorf("cannot create basic request: %w", err)
	}

	slog.Info("request", "request", request)

	client := &http.Client{}
	response, err := client.Do(request)
	if err != nil {
		return fmt.Errorf("sending request: %w", err)
	}

	responseBody, err := io.ReadAll(response.Body)
	if err != nil {
		return fmt.Errorf("cannot read response body: %w", err)
	}

	// todo display stuff
	fmt.Println(response.StatusCode)
	fmt.Println(string(responseBody))

	return nil
}

func createRequest(head string, headers http.Header, body io.ReadCloser) (*http.Request, error) {
	lines := strings.Split(head, "\n")
	if len(lines) < 1 {
		return nil, fmt.Errorf("too few lines in the header of the file: %s", head)
	}

	essentials := strings.Split(lines[0], " ")

	if len(essentials) < 2 {
		return nil, errors.New("method or url is missing")
	}

	method, urlRaw := essentials[0], essentials[1]

	parsedUrl, err := url.Parse(urlRaw)
	if err != nil {
		return nil, fmt.Errorf("cannot parse url: %s", urlRaw)
	}

	request, err := http.NewRequest(method, parsedUrl.String(), body)
	if err != nil {
		return nil, fmt.Errorf("cannot create a request: %w", err)
	}

	for k := range headers {
		request.Header.Add(k, headers.Get(k))
	}

	return request, nil
}

func attachHeaders(request *http.Request, headers []string) {
	for _, header := range headers {
		key, value, ok := strings.Cut(header, ":")
		if !ok {
			slog.Warn("cannot parse header", "header", header)
		}

		request.Header.Add(strings.TrimSpace(key), strings.TrimSpace(value))
	}
}

func getBody(contentType, body string) (io.ReadCloser, error) {
	if contentType == "" {
		return nil, nil
	}

	switch contentType {
	case "application/json":
		return getJSONBody(body), nil
	default:
		slog.Warn("unknown content-type", "content-type", contentType)
		return nil, nil
	}
}

func getJSONBody(body string) io.ReadCloser {
	return io.NopCloser(strings.NewReader(body))
}

func getHeaders(headersRaw string) http.Header {
	result := make(http.Header)

	for _, header := range strings.Split(headersRaw, "\n") {
		key, value, ok := strings.Cut(header, ":")
		if !ok {
			slog.Warn("cannot parse header", "header", header)
			continue
		}

		result.Add(strings.TrimSpace(key), strings.TrimSpace(value))
	}

	return result
}
