#!/bin/sh
set -x

for k in asdf zxcv 28F41A2800008091
do
  for i in $(seq 5)
  do
    r=$(rand -f -s $(dd status=none if=/dev/urandom bs=1 count=8 | sum | awk '{print $1}'))
    t=$(echo "$r*20"|bc)
    echo -n "$k $t" \
      | coap-client -m post -f - coap://localhost/store_temp
  done
done

coap-client -m get coap://localhost/dump
coap-client -m get coap://localhost/avg_out
echo -n qwerty | coap-client -m post -f - coap://localhost/set_outsensor
coap-client -m get coap://localhost/avg_out
echo -n asdf | coap-client -m post -f - coap://localhost/set_outsensor
coap-client -m get coap://localhost/avg_out
echo -n zxcv | coap-client -m post -f - coap://localhost/set_outsensor
coap-client -m get coap://localhost/avg_out
echo -n 28F41A2800008091 \
    | coap-client -m post -f - coap://localhost/set_outsensor
coap-client -m get coap://localhost/avg_out

exit 0
# EOF
