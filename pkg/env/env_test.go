package env

import (
	"github.com/stretchr/testify/assert"
	"testing"
)

func TestParse(t *testing.T) {
	envs, sslConfigs, err := Parse("../../testdata/http-client.env.json")
	assert.NoError(t, err)

	env := envs["dev"]
	assert.NotNil(t, env)

	sslConfig := sslConfigs["dev"]
	assert.Equal(t, "../echo/certs/localhost+2.pem", sslConfig.ClientCertificate)
	assert.Equal(t, "../echo/certs/localhost+2-key.pem", sslConfig.ClientCertificateKey)
}
