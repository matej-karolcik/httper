package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

func main() {
	http.HandleFunc("/bearer", func(w http.ResponseWriter, r *http.Request) {
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
	})

	http.HandleFunc("/basic-auth", func(w http.ResponseWriter, r *http.Request) {
		if u, p, ok := r.BasicAuth(); !ok || u != "foo" || p != "bar" {
			w.Header().Set("WWW-Authenticate", `Basic realm="Restricted"`)
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		_, _ = fmt.Fprintln(w, "Authorized")
	})
	http.HandleFunc("/json", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
			return
		}

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
	})

	http.HandleFunc("/form-data", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			w.WriteHeader(http.StatusMethodNotAllowed)
			return
		}

		reader, err := r.MultipartReader()
		if err == nil && reader != nil {
			for {
				part, err := reader.NextPart()
				if err != nil {
					break
				}

				_, _ = fmt.Fprintf(w, "Part: %s, %s\n", part.FormName(), part.FileName())

				content, err := io.ReadAll(part)
				if err != nil {
					_, _ = fmt.Fprintf(w, "Error reading part: %s\n", err)
					continue
				}

				_, _ = fmt.Fprintf(w, "Content-length: %d\n", len(content))
			}
		}
	})

	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {})

	const addr = ":8080"
	fmt.Println("Listening on", addr)

	if err := http.ListenAndServeTLS(addr, "./certs/localhost+1.pem", "./certs/localhost+1-key.pem", nil); err != nil {
		panic(err)
	}
}
