package main

import (
	"fmt"
	"io"
	"net/http"
)

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
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

				_, _ = fmt.Fprintf(w, "Content: %s\n", content)
			}
		}

		w.WriteHeader(http.StatusOK)
	})

	const addr = ":8080"
	fmt.Println("Listening on", addr)

	if err := http.ListenAndServeTLS(addr, "./certs/localhost+1.pem", "./certs/localhost+1-key.pem", nil); err != nil {
		panic(err)
	}
}
