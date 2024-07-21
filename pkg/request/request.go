package request

import (
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"net/textproto"
	"net/url"
	"regexp"
	"strings"
)

var splitRegex = regexp.MustCompile(`(?m)^###`)

func Create(content, wd string) ([]*http.Request, error) {
	requests := make([]*http.Request, 0)

	for _, part := range splitRequests(content) {
		part = strings.TrimSpace(part)
		if part == "" {
			continue
		}

		request, err := parse(part, wd)
		if err != nil {
			return nil, fmt.Errorf("parsing request: %w", err)
		}

		requests = append(requests, request)
	}

	return requests, nil
}

func splitRequests(content string) []string {
	return splitRegex.Split(content, -1)
}

func splitRequest(content string) (essentials, headers, body string) {
	var head string
	head, body, _ = strings.Cut(content, "\n\n")
	essentials, headers, _ = strings.Cut(head, "\n")

	essentials = strings.TrimSpace(essentials)
	headers = strings.TrimSpace(headers)
	body = strings.TrimSpace(body)

	return
}

func parse(content, wd string) (*http.Request, error) {
	essentialsRaw, headersRaw, bodyRaw := splitRequest(content)

	headers := parseHeaders(headersRaw)
	body, err := parseBody(
		headers.Get("Content-Type"),
		bodyRaw,
		wd,
	)
	if err != nil {
		return nil, fmt.Errorf("getting request body: %w", err)
	}

	lines := strings.Split(essentialsRaw, "\n")
	if len(lines) < 1 {
		return nil, fmt.Errorf("too few lines in the header of the file: %s", essentialsRaw)
	}

	essentials := strings.Split(lines[0], " ")

	var method, urlRaw string

	if len(essentials) < 2 {
		method = http.MethodGet
		urlRaw = essentials[0]
	} else {
		method, urlRaw = essentials[0], essentials[1]
	}

	parsedUrl, err := url.Parse(urlRaw)
	if err != nil {
		return nil, fmt.Errorf("cannot parse url: %s", urlRaw)
	}

	request, err := http.NewRequest(method, parsedUrl.String(), body)
	if err != nil {
		return nil, fmt.Errorf("cannot create a request: %w", err)
	}

	transferHeaders(request, headers)

	return request, nil
}

func transferHeaders(request *http.Request, headers textproto.MIMEHeader) {
	for k := range headers {
		value := headers.Get(k)
		if strings.ToLower(k) != "authorization" {
			request.Header.Add(k, value)
		}

		parts := strings.Split(value, " ")

		if len(parts) < 2 {
			continue
		}

		if strings.ToLower(parts[0]) != "basic" {
			request.Header.Add(k, value)
			continue
		}

		var password string
		if len(parts) > 2 {
			password = parts[2]
		}

		request.SetBasicAuth(parts[1], password)
	}
}

func parseBody(contentType, body, wd string) (io.Reader, error) {
	if contentType == "" {
		return nil, nil
	}

	contentType, boundary := splitContentType(contentType)

	switch contentType {
	case "application/json":
		return getJSONBody(body), nil
	case "multipart/form-data":
		return getFormDataBody(boundary, body, wd)
	default:
		slog.Warn("unknown content-type", "content-type", contentType)
		return nil, nil
	}
}

func getJSONBody(body string) io.Reader {
	return strings.NewReader(body)
}

func parseHeaders(headersRaw string) textproto.MIMEHeader {
	result := make(textproto.MIMEHeader)

	if strings.TrimSpace(headersRaw) == "" {
		return result
	}

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
