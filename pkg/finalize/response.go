package finalize

import (
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"text/tabwriter"
	"time"
)

func Response(
	response *http.Response,
	duration time.Duration,
	save, verbose bool,
) {
	if save {
		if err := saveResponse(response); err != nil {
			slog.Error("saving response", "err", err)
		}
	}

	w := tabwriter.NewWriter(os.Stdout, 20, 20, 1, ' ', tabwriter.Debug)

	_, _ = fmt.Fprintln(w)
	_, _ = fmt.Fprintf(w, "Status\t%d\n", response.StatusCode)
	_, _ = fmt.Fprintf(w, "Duration\t%s\n", duration)
	_, _ = fmt.Fprintf(w, "Content-Length\t%d\n", response.ContentLength)

	if err := w.Flush(); err != nil {
		slog.Error("flushing tabwriter", "err", err)
	}
}
