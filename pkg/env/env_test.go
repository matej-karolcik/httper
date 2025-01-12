package env

import (
	"github.com/stretchr/testify/assert"
	"testing"
)

func TestParse(t *testing.T) {
	envs, err := Parse("../../testdata/http-client.env.json")
	assert.NoError(t, err)

	env := envs["dev"]
	assert.NotNil(t, env)
}
