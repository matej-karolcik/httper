package finalize

import (
	"fmt"
	"github.com/gabriel-vasile/mimetype"
	"io"
	"mime"
	"net/http"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

func saveResponse(response *http.Response) error {
	prefix, err := getFilePrefix()
	if err != nil {
		return fmt.Errorf("getting file prefix: %w", err)
	}

	extension := getExtension(response)

	file, err := os.Create(path.Join(prefix, getFilename(response.StatusCode, extension)))
	if err != nil {
		return fmt.Errorf("creating file: %w", err)
	}

	defer func() {
		_ = file.Close()
	}()

	if _, err = io.Copy(file, response.Body); err != nil {
		return fmt.Errorf("copying response body: %w", err)
	}

	return nil
}

func getFilePrefix() (string, error) {
	const prefix = ".idea/httpRequests"

	// current directory has .idea dir
	if finfo, err := os.Stat(".idea"); err == nil && finfo.IsDir() {
		if _, err = os.Stat(prefix); err != nil {
			if err = os.MkdirAll(prefix, 0755); err != nil {
				return "", fmt.Errorf("creating dir: %w", err)
			}
		}

		return prefix, nil
	}

	// parent directory has .idea dir
	wd, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("getting working directory: %w", err)
	}

	parts := strings.Split(wd, string(filepath.Separator))
	for i, part := range parts {
		if part == ".idea" {
			return string(filepath.Separator) + strings.Join(parts[:i+1], string(filepath.Separator)), nil
		}
	}

	// fallback to current directory
	if err = os.MkdirAll(prefix, 0755); err != nil {
		return "", fmt.Errorf("creating dir: %w", err)
	}

	return prefix, nil
}

func getExtension(response *http.Response) string {
	mimeType, err := mimetype.DetectReader(response.Body)
	if err == nil {
		return mimeType.Extension()
	}

	exts, err := mime.ExtensionsByType(response.Header.Get("Content-Type"))
	if err == nil && len(exts) > 0 {
		sort.Sort(sort.Reverse(sort.StringSlice(exts)))
		return exts[0]
	}

	return ".txt"
}

func getFilename(statusCode int, ext string) string {
	return fmt.Sprintf(
		"%s.%d%s",
		time.Now().Format("2006-01-02T150405"),
		statusCode,
		ext,
	)
}
