Simple alpine linux packages server you can use to consume and update your private packages in CI/CD pipelines.

[TOC]

# Presentation

reposerve is a simple [alpine linux](https://alpinelinux.org/) packages http server you can use to consume and update
your private packages in CI/CD pipelines. Written in async rust, it acts as a simple static file server with an API
protected by a JWT Token to easily upload new packages.

You can define arbitrary webhooks accessible at `/webhook/webhook_name` to lauch predefined scripts (force a sign of all
packages for example).

![reposerve](reposerve.png?raw=true "Reposerve")

# Usage

```
reposerve 0.4.3

Usage: reposerve [-c <config>] [-v] [-a <addr>]

Extract latest projects archives from a gitlab server

Options:
  -c, --config      configuration file containing projects and gitlab connection
                    parameters
  -v, --verbose     more detailed output
  -a, --addr        addr:port to bind to
  --help            display usage information
```

# Uploading files

Uploading packages is easy and can be done with curl. You just have to get a JWT token. `version` is the alpine version
(edge), `repo` is the repository name (main) and `arch` (x86_64) are all optional. You can submit multiple apk
file.

This is for example a script that will upload all files to a reposerve service on $HOST under gitlab (using the job JWT
token).

```sh
#!/bin/sh

. /etc/os-release

VERSION=${VERSION_ID%.*}
REPO=$(basename $(dirname $(pwd)))
DIR=${1:-/tmp/apkdeploy}

for arch in $(find $DIR -name APKINDEX.tar.gz); do
	ARCH=$(basename $(dirname $arch))
	args="-H 'Authorization: Bearer $CI_JOB_JWT' -F 'version=$VERSION' -F 'repo=$REPO' -F 'arch=$ARCH' "

	for file in $(find $DIR -name '*.apk'); do
		args=$args"-F file=@$(basename $file) "
	done

	(cd $(dirname $arch) && eval curl $args https://$HOST/upload)
done
```

After upload, the index is automatically reconstructed and signed by reposerve (no need to define extra webhook).

# Configuration

A `jwt` configuration has to be provided in the configuration for the `/upload` API point to be activated. You
need to give the url to retrieve the public keys (`jwks`) that sign the JWT Tokens and a map of claims with their
expected values the token must comply with, to be allowed to upload files to the repository.

```yaml
dir: /home/packager/packages
tls:
  crt: /var/run/secrets/reposerve/tls.crt
  key: /var/run/secrets/reposerve/tls.key
jwt:
  jwks: https://gitlab.com/-/jwks
  claims:
    iss: gitlab.com
webhooks:
  sign: /usr/bin/apk_sign.sh
```

`tls`, `jwt` and `webhooks` are all optional.
