"""Zip the active installation for the Android app (stored, with a hash manifest)."""
from pathlib import Path
import argparse,hashlib,io,json,os,sys,zipfile
from datetime import datetime,timezone
ROOT=Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:sys.path.insert(0,str(ROOT))
from tools.asset_pipeline.versions import installed
MANIFEST='android-manifest.json'
SKIP_DIRS={'__pycache__','cache','tmp','temp','work'}
CHUNK=1<<20

def skipped(rel,exclude_speech):
    name=rel.name.lower()
    if name==MANIFEST or name.endswith(('.log','.tmp','.new','.pyc')):return True
    if any(part.lower() in SKIP_DIRS for part in rel.parts[:-1]):return True
    # Speech index JSON stays; only decoded speech audio is droppable.
    return exclude_speech and 'speech' in [p.lower() for p in rel.parts[:-1]] and name!='livingworld.json' and rel.suffix!='.json'

def patched_audio_manifest(path):
    """Audio manifest bytes with every decoded speech audio entry set to None (what the engine reads as no decoded speech)."""
    manifest=json.loads(path.read_text(encoding='utf-8'))
    for entry in (manifest.get('speech') or {}).values():
        if isinstance(entry,dict):entry['audio']=None
    return json.dumps(manifest,indent=1).encode('utf-8')

def export(base,output,report=print,exclude_speech=False):
    found=installed(Path(base))
    if found is None:raise RuntimeError('No installation found in '+str(base))
    root,marker=found
    files=sorted(p for p in root.rglob('*') if p.is_file() and not skipped(p.relative_to(root),exclude_speech))
    if not files:raise RuntimeError('Installation is empty')
    total=sum(p.stat().st_size for p in files)
    report(f'Exporting {len(files)} files ({total/2**20:.0f} MiB)')
    output=Path(output);tmp=output.with_name(output.name+'.part')
    entries={};done=0
    try:
        with zipfile.ZipFile(tmp,'w',zipfile.ZIP_STORED,allowZip64=True) as zf:
            for i,path in enumerate(files):
                rel=path.relative_to(root).as_posix();digest=hashlib.sha256();size=0
                # Small JSON, patched in memory; everything else streams.
                src=io.BytesIO(patched_audio_manifest(path)) if exclude_speech and path.name=='audio_manifest.json' else path.open('rb')
                with src,zf.open(zipfile.ZipInfo('installation/'+rel,(1980,1,1,0,0,0)),'w',force_zip64=True) as dst:
                    while chunk:=src.read(CHUNK):
                        digest.update(chunk);dst.write(chunk);size+=len(chunk)
                entries[rel]={'size':size,'sha256':digest.hexdigest()};done+=size
                if i%50==0 or i==len(files)-1:report(f'{i+1}/{len(files)} files, {done*100//max(total,1)}%')
            manifest={'format':1,'created':datetime.now(timezone.utc).isoformat(),
                      'installation_id':root.name,'pipelines':marker.get('pipelines',{}),'files':entries}
            zf.writestr(zipfile.ZipInfo('installation/'+MANIFEST,(1980,1,1,0,0,0)),json.dumps(manifest,indent=1))
        os.replace(tmp,output)
    finally:
        tmp.unlink(missing_ok=True)
    report('Export complete: '+str(output))
    return output

def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base',type=Path,required=True,help='data dir containing installation.json')
    parser.add_argument('--output',type=Path,default=Path('skate3-android.zip'))
    parser.add_argument('--exclude-speech',action='store_true',help='drop decoded speech audio and patch audio_manifest.json to match')
    args=parser.parse_args(argv)
    export(args.base,args.output,print,args.exclude_speech)
    return 0

if __name__=='__main__':raise SystemExit(main())
