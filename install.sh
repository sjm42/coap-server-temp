#!/bin/sh

set -x
set -e

PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export PATH

tgt=$HOME/coap-server/bin
rsync -var target/release/coap-server $tgt/

exit 0
# EOF
