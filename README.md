# reposerve

reposerve is a simple [alpine linux](https://alpinelinux.org/) packages server you can use to consume and update
your private packages in CI/CD pipelines. Written in async rust, it acts as a simple static file server
with an API, protected by a Token, to easily upload new file. You can define arbitrary webhooks to launch
predefined action (like signing index).

![reposerve](reposerve.png?raw=true "Reposerve")

## Usage

```
Usage: reposerve -c <config> [-v]

Extract latest projects archives from a gitlab server

Options:
  -c, --config      configuration file containing projects and gitlab connection
                    parameters
  -v, --verbose     more detailed output
  --help            display usage information

```

## Uploading files

Uploading packages is easy and can be done with curl. You just have to get a token. `version` is the alpine version
(edge), `repo` is the repository name (main) and `arch` (x86_64) are all optional. You can submit multiple apk
file in the


This is for example a script that needs to executed on repository root of an alpine host that upload all files
to reposerve identified by $HOST.

```sh
#!/bin/sh

. /etc/os-release

VERSION=${VERSION_ID%.*}
REPO=$(basename $(dirname $(pwd)))
DIR=${1:-/tmp/apkdeploy}

for arch in $(find $DIR -name APKINDEX.tar.gz); do
	ARCH=$(basename $(dirname $arch))
	args="-H 'token: $TOKEN' -F 'version=$VERSION' -F 'repo=$REPO' -F 'arch=$ARCH' "

	for file in $(find $DIR -name '*.apk'); do
		args=$args"-F file=@$(basename $file) "
	done

	(cd $(dirname $arch) && eval curl $args https://$HOST/upload)
done
```

After upload, the index is automatically reconstructed and signed by reposerve.

# Configuration

```yaml
dir: /home/packager/packages
token: xxxxxxxxxx
webhooks:
  sign: /usr/bin/apk_sign.sh
```
