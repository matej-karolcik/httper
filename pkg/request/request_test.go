package request

import (
	"github.com/stretchr/testify/assert"
	"os"
	"testing"
)

const formContent = `POST https://localhost:8080/form-data
Content-Type: multipart/form-data; boundary=foo

--foo
Content-Disposition: form-data; name="image"; filename="Cargo.lock"
Content-Type: application/octet-stream

< ../Cargo.lock
--foo
content-Disposition: form-data; name="title"
Content-Type: text/plain

test text

foobar
--foo--`

func TestSplitRequests(t *testing.T) {
	wd, _ := os.Getwd()
	t.Log(wd)
	content, err := os.ReadFile("../../testdata/three.http")
	assert.NoError(t, err)

	parts := splitRequests(string(content))

	assert.Len(t, parts, 3)
}

func TestSplitRequest(t *testing.T) {
	essentials, headers, body := splitRequest(formContent)

	assert.Equal(t, "POST https://localhost:8080/form-data", essentials)
	assert.Equal(t, "Content-Type: multipart/form-data; boundary=foo", headers)
	assert.Equal(t, `--foo
Content-Disposition: form-data; name="image"; filename="Cargo.lock"
Content-Type: application/octet-stream

< ../Cargo.lock
--foo
content-Disposition: form-data; name="title"
Content-Type: text/plain

test text

foobar
--foo--`, body)
}
