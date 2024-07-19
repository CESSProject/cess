#!/usr/bin/env bash

set -e

if [ "$SGX" -eq 1 ] && [ "$SKIP_AESMD" -eq 0 ]; then
  echo "Starting AESMD"

  if test -f "/opt/conf/aesmd.conf"; then
    echo "Found custom aesmd.conf, override the default."
    cp /opt/conf/aesmd.conf /etc/
  fi

  if test -f "/opt/conf/sgx_default_qcnl.conf"; then
    echo "Found custom sgx_default_qcnl.conf, override the default."
    cp /opt/conf/sgx_default_qcnl.conf /etc/
  fi
  
  /bin/mkdir -p /var/run/aesmd/
  /bin/chown -R aesmd:aesmd /var/run/aesmd/
  /bin/chmod 0755 /var/run/aesmd/
  /bin/chown -R aesmd:aesmd /var/opt/aesmd/
  /bin/chmod 0750 /var/opt/aesmd/

  LD_LIBRARY_PATH=/opt/intel/sgx-aesm-service/aesm /opt/intel/sgx-aesm-service/aesm/aesm_service --no-daemon &

  if [ ! "${SLEEP_BEFORE_START:=0}" -eq 0 ]
  then
    echo "Waiting for device. Sleep ${SLEEP_BEFORE_START}s"

    sleep "${SLEEP_BEFORE_START}"
  fi
fi

if [ "$RA_METHOD" = "dcap" ]; then
  if [ -e "/opt/ceseal/releases/current/dcap-ver" ]; then
    echo "Dcap version found"
    rm -rf /opt/ceseal/releases/current/dcap-ver/data
    mv /opt/ceseal/releases/current/dcap-ver/* /opt/ceseal/releases/current/
    rm -rf /opt/ceseal/releases/current/dcap-ver
  fi
else
  if [ -e "/opt/ceseal/releases/current/epid-ver" ]; then
    echo "Epid version found"
    rm -rf /opt/ceseal/releases/current/epid-ver/data
    mv /opt/ceseal/releases/current/epid-ver/* /opt/ceseal/releases/current/
    rm -rf /opt/ceseal/releases/current/epid-ver
  fi
fi


./handover --ra-type=$RA_METHOD
cd /opt/ceseal/releases/current && SKIP_AESMD=1 ./start.sh