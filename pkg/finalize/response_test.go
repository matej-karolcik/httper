package finalize

import (
	"github.com/stretchr/testify/assert"
	"testing"
	"time"
)

func TestFormat(t *testing.T) {
	t0, err := time.Parse("2006-01-02T150405", "2006-01-02T150405")
	assert.NoError(t, err)
	str := t0.Format("2006-01-02T150405")

	assert.Equal(t, "2006-01-02T150405", str)
}
