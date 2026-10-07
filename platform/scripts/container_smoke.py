"""Disposable local OCI/Compose smoke; never publishes or deploys an image."""
import os
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

repo=Path(__file__).resolve().parents[2]
project='contingram-smoke-'+str(os.getpid())
compose=['docker','compose','-p',project,'-f','platform/compose.yaml']
container=None
try:
    subprocess.run([*compose,'config','--quiet'],cwd=repo,check=True)
    subprocess.run([*compose,'up','-d','--wait','--wait-timeout','90'],cwd=repo,check=True)
    with tempfile.TemporaryDirectory(prefix='contingram-issuer-') as tmp:
        key=Path(tmp)/'key.pem';pub=Path(tmp)/'public.pem'
        subprocess.run(['openssl','genpkey','-algorithm','RSA','-pkeyopt','rsa_keygen_bits:2048','-out',str(key)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        subprocess.run(['openssl','pkey','-in',str(key),'-pubout','-out',str(pub)],check=True)
        pub.chmod(0o644)
        args=['docker','run','-d','--network',project+'_default','-p','127.0.0.1::8080','--mount',f'type=bind,source={pub},target=/app/public.pem,readonly']
        for name,value in {'DATABASE_URL':'jdbc:postgresql://postgres:5432/contingram','DATABASE_USER':'contingram','DATABASE_PASSWORD':'local-development-only','KAFKA_BOOTSTRAP_SERVERS':'kafka:9092','OIDC_ISSUER':'https://issuer.example','OIDC_PUBLIC_KEY':'file:/app/public.pem'}.items():
            args.extend(['-e',name+'='+value])
        container=subprocess.check_output([*args,'contingram-platform:local'],text=True).strip()
        address=subprocess.check_output(['docker','port',container,'8080'],text=True).strip()
        for attempt in range(60):
            try:
                urllib.request.urlopen('http://'+address+'/v1/intents',timeout=2)
            except urllib.error.HTTPError as error:
                assert error.code==401,error.code
                break
            except (urllib.error.URLError,TimeoutError,ConnectionError):
                time.sleep(1)
        else:
            raise RuntimeError('Container HTTP listener did not become ready')
        subprocess.run(['docker','exec',container,'/app/contingram','demo'],check=True)
        print('OCI service boots with Compose dependencies; unauthenticated HTTP rejected; Linux verifier demo passes')
finally:
    if container:
        subprocess.run(['docker','logs',container],check=True)
        subprocess.run(['docker','rm','-f',container],check=True)
    subprocess.run([*compose,'down','--volumes'],cwd=repo,check=True)
