package finish

import (
	"fmt"
	"io"
	"log/slog"
	"mime"
	"net/http"
	"os"
	"path"
	"sort"
	"text/tabwriter"
	"time"
)

func Handle(
	response *http.Response,
	duration time.Duration,
	save bool,
) {
	w := tabwriter.NewWriter(os.Stdout, 20, 20, 1, ' ', tabwriter.Debug)

	_, _ = fmt.Fprintln(w)
	_, _ = fmt.Fprintf(w, "Status\t%d\n", response.StatusCode)
	_, _ = fmt.Fprintf(w, "Duration\t%s\n", duration)
	_, _ = fmt.Fprintf(w, "Content-Length\t%d\n", response.ContentLength)

	if save {
		if err := saveResponse(response); err != nil {
			slog.Error("saving response", "err", err)
		}
	}

	if err := w.Flush(); err != nil {
		slog.Error("flushing tabwriter", "err", err)
	}
}

func saveResponse(response *http.Response) error {
	var prefix, ext string
	if finfo, err := os.Stat(".idea"); err == nil && finfo.IsDir() {
		prefix = ".idea/httpRequests"
		if _, err = os.Stat(prefix); err != nil {
			if err = os.MkdirAll(prefix, 0755); err != nil {
				slog.Error("creating dir", "err", err)
				prefix = ""
			}
		}
	}

	exts, err := mime.ExtensionsByType(response.Header.Get("Content-Type"))
	if err != nil || len(exts) == 0 {
		ext = ".txt"
	} else {
		// todo maybe a mapping would be better
		sort.Sort(sort.Reverse(sort.StringSlice(exts)))
		ext = exts[0]
	}

	file, err := os.Create(path.Join(prefix, getFilename(response.StatusCode, ext)))
	if err != nil {
		return fmt.Errorf("creating file: %w", err)
	}

	defer func() {
		_ = file.Close()
	}()

	if _, err = io.Copy(file, response.Body); err != nil {
		return fmt.Errorf("copying response body: %w", err)
	}

	return response.Body.Close()
}

func getFilename(statusCode int, ext string) string {
	return fmt.Sprintf(
		"%s.%d%s",
		time.Now().Format("2006-01-02T150405"),
		statusCode,
		ext,
	)
}
