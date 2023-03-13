Simple alpine Linux packages server you can use to consume and update your private packages in CI/
CD pipelines.

[TOC]

# Presentation

`reposerve` is a simple [alpine Linux](https://alpinelinux.org/) packages HTTP(S) server you can
use to consume and update your private packages in CI/CD pipelines. Written in async rust, it acts
as a simple static file server with an API protected by a JWT Token to easily upload new packages.

You can follow the tutorial [Building Alpine Linux packages inside a
container](https:// itsufficient.me/blog/alpine-build) to understand
what issue it tries to resolve and read [A better way to build containers
images](https://itsufficient.me/blog/alpine-container#cicd-to-rule-them-all) to see how I
integrated alpine packages in the fabric of container images with CI/CD pipelines.

![reposerve](reposerve.png?raw=true "Reposerve")

# Usage

```
reposerve 0.7.0

Usage: reposerve [-c <config>] [-d] [-v] [-l <addr>] [-L <addrs>] [-S]

Simple Alpine Linux packages server

Options:
  -c, --config      configuration file (/etc/reposerve.yaml)
  -d, --dev         dev mode: enable /webhook and /upload without jwt (false)
  -v, --verbose     more detailed output (false)
  -l, --addr        addr:port to bind to (0.0.0.0:8080) without tls
  -L, --addrs       addr:port to bind to (0.0.0.0:8443) when tls is used
  -S, --secure      only bind to tls (when tls config is present in
                    configuration file)
  --help            display usage information
```

# Uploading files

Uploading packages is easy and can be done with curl. You just have to get a JWT token. `version`
is the alpine version (edge), `repo` is the repository name (main) and `arch` (x86_64) are all
optional. You can submit multiple apk file.

This is for example a script that will upload all files to a `reposerve` service on $HOST under GitLab
(using the job JWT token).

```sh
#!/bin/sh

. /etc/os-release

VERSION=${VERSION_ID%.*}
REPO="$(basename $(dirname $(pwd)))"
DIR=${1:-/tmp/apkdeploy}

for arch in "$(find $DIR -name APKINDEX.tar.gz)"; do
	ARCH="$(basename $(dirname $arch))"
	args="-H 'Authorization: Bearer $CI_JOB_JWT_V2' -F 'version=$VERSION' -F 'repo=$REPO' -F 'arch=$ARCH' "

	for file in "$(find $DIR -name '*.apk')"; do
		args="${args}-F file=@$(basename $file) "
	done

	(cd "$(dirname $arch)" && eval curl $args https://$HOST/upload)
done
```

After an upload, the index is automatically reconstructed and signed by `reposerve` (no need to define
extra webhook).

# Configuration

A `jwt` configuration has to be provided in the configuration for the `/upload` and `/webhooks`
unless the dev mode is activated. You need to give the URL to retrieve the public keys (`jwks`) that
sign the JWT Tokens and a map of claims with their expected values the token must comply with, to be
allowed to upload files to the repository.

`redirect` allows to configure an automatic HTTPS redirection per protocol to deal with
differences in dual stack deployment. Generally you use a reverse proxy with IPv4 while you
expose your service directly with IPv6. Because some reverse proxy don't handle TLS encrypted
upstream you can't send them a redirect, so you can disable it in that specific case.

When `hsts` is provided, then [HTTP Strict Transport
Security](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security)
are sent according to the parameters.

```yaml
dir: /home/packager/packages
tls:
  crt: /var/run/secrets/reposerve/tls.crt
  key: /var/run/secrets/reposerve/tls.key
  # perform a redirection to https
  redirect:
    port: 443 # optional
    protocols: both # both | ipv4 | ipv6 | none
  # Strict-Transport-Security header
  hsts:
    duration: 300 # duration in s
    include_subdomains: true
    preload: false
# protect /upload and /webhooks with a JWT token
jwt:
  jwks: https://gitlab.com/-/jwks
  claims:
    iss: gitlab.com
# webhooks definition under /webhooks/..
webhooks:
  sign: /usr/bin/apk_sign.sh
```

`tls`, `redirect`, `hsts`, `jwt` and `webhooks` are all optional. Be careful when deploying
in production