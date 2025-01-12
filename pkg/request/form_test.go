package request

import (
	"bytes"
	"github.com/stretchr/testify/assert"
	"io"
	"os"
	"testing"
)

func TestGetFormDataBody(t *testing.T) {
	//reader := getFormDataBody("foo", formContent)
}

func TestGetFiles(t *testing.T) {
	bodyRaw := `< ../../testdata/bearer.http
< ../../testdata/get.http`

	wd, err := os.Getwd()
	assert.NoError(t, err)
	r, err := getFiles(bodyRaw, wd)
	assert.NoError(t, err)

	actual := new(bytes.Buffer)

	if _, err = io.Copy(actual, r); err != nil {
		t.Fatal(err)
	}

	expected := `GET https://localhost:8080/bearer
Authorization: Bearer 42069

GET https://localhost:8080/?{{param}}&query&param1=foobar
`

	assert.Equal(t, expected, actual.String())
}
