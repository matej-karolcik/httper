package handler

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

func Bearer(w http.ResponseWriter, r *http.Request) {
	const bearerPrefix = "Bearer "
	bearer := r.Header.Get("Authorization")

	if !strings.HasPrefix(bearer, bearerPrefix) {
		w.WriteHeader(http.StatusUnauthorized)
		return
	}

	if strings.TrimPrefix(bearer, bearerPrefix) != "42069" {
		w.WriteHeader(http.StatusForbidden)
		return
	}

	_, _ = fmt.Fprintln(w, "Authorized")
}

func BasicAuth(w http.ResponseWriter, r *http.Request) {
	if u, p, ok := r.BasicAuth(); !ok || u != "foo" || p != "bar" {
		w.Header().Set("WWW-Authenticate", `Basic realm="Restricted"`)
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	_, _ = fmt.Fprintln(w, "Authorized")
}

func JsonBody(w http.ResponseWriter, r *http.Request) {
	content, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	if err = json.NewDecoder(bytes.NewReader(content)).Decode(&struct{}{}); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	_, _ = fmt.Fprintf(w, "Content-length: %d\n", len(content))
}

func FormData(w http.ResponseWriter, r *http.Request) {
	reader, err := r.MultipartReader()
	if err == nil && reader != nil {
		for {
			part, err := reader.NextPart()
			if err != nil {
				break
			}

			_, _ = fmt.Fprintf(w, "Part: %s, '%s'\n", part.FormName(), part.FileName())

			content, err := io.ReadAll(part)
			if err != nil {
				_, _ = fmt.Fprintf(w, "Error reading part: %s\n", err)
				continue
			}

			if part.FileName() == "" {
				_, _ = fmt.Fprintln(w, string(content))
			}

			if r.URL.Query().Has("debug") {
				_, _ = fmt.Fprintln(w)
			}

			if r.URL.Query().Has("headers") {
				for k, v := range part.Header {
					_, _ = fmt.Fprintf(w, "%s: %s\n", k, v)
				}
			}

			_, _ = fmt.Fprintf(w, "Content-length: %d\n", len(content))
		}
	}
}

func CatchAll(w http.ResponseWriter, r *http.Request) {
	if r.URL.Query().Has("query") {
		for k, v := range r.URL.Query() {
			_, _ = fmt.Fprintf(w, "%s: %s\n", k, v)
		}
	}
}
