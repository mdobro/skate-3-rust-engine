"""Android export zip layout, manifest and speech exclusion."""
import hashlib
import json
import tempfile
import unittest
import zipfile
from pathlib import Path
from tools import export_android


def fake(base):
    ident='a'*32
    root=base/'installations'/ident
    files={'maps/a.skate':b'map','maps.json':b'[]','settings/x.json':b'{}','assets/private/game.json':b'{}',
           'assets/audio/speech/livingworld.json':b'{}','assets/audio/speech/livingworld/1.wav':b'riff',
           'setup.log':b'log','assets/cache/t.bin':b'c',
           'assets/private/audio/audio_manifest.json':json.dumps({'speech':{'livingworld':{'index':'speech/livingworld.json','audio':'speech/livingworld'}},'banks':{}}).encode()}
    for rel,data in files.items():
        (root/rel).parent.mkdir(parents=True,exist_ok=True);(root/rel).write_bytes(data)
    (base/'installation.json').write_text(json.dumps({'version':1,'directory':'installations/'+ident,'pipelines':{'core':'1'}}))
    return ident


class ExportAndroid(unittest.TestCase):
    def run_export(self,**kw):
        temp=tempfile.TemporaryDirectory();self.addCleanup(temp.cleanup)
        base=Path(temp.name)/'data';base.mkdir();ident=fake(base)
        out=Path(temp.name)/'out.zip'
        export_android.export(base,out,lambda _:None,**kw)
        return ident,out

    def test_layout_manifest_and_order(self):
        ident,out=self.run_export()
        with zipfile.ZipFile(out) as zf:
            names=zf.namelist()
            self.assertEqual(names[-1],'installation/android-manifest.json')
            self.assertTrue(all(i.compress_type==zipfile.ZIP_STORED for i in zf.infolist()))
            manifest=json.loads(zf.read(names[-1]))
            self.assertEqual((manifest['format'],manifest['installation_id'],manifest['pipelines']),(1,ident,{'core':'1'}))
            self.assertIn('maps/a.skate',manifest['files'])
            self.assertNotIn('setup.log',manifest['files'])
            self.assertNotIn('assets/cache/t.bin',manifest['files'])
            for rel,meta in manifest['files'].items():
                data=zf.read('installation/'+rel)
                self.assertEqual(meta,{'size':len(data),'sha256':hashlib.sha256(data).hexdigest()})
            self.assertEqual(len(names),len(manifest['files'])+1)

    def test_speech_included_by_default(self):
        _,out=self.run_export()
        with zipfile.ZipFile(out) as zf:
            self.assertIn('installation/assets/audio/speech/livingworld/1.wav',zf.namelist())
            m=json.loads(zf.read('installation/assets/private/audio/audio_manifest.json'))
            self.assertEqual(m['speech']['livingworld']['audio'],'speech/livingworld')

    def test_exclude_speech_patches_audio_manifest(self):
        _,out=self.run_export(exclude_speech=True)
        with zipfile.ZipFile(out) as zf:
            names=zf.namelist()
            self.assertIn('installation/assets/audio/speech/livingworld.json',names)
            self.assertNotIn('installation/assets/audio/speech/livingworld/1.wav',names)
            raw=zf.read('installation/assets/private/audio/audio_manifest.json')
            m=json.loads(raw)
            self.assertIsNone(m['speech']['livingworld']['audio'])
            self.assertEqual(m['speech']['livingworld']['index'],'speech/livingworld.json')
            meta=json.loads(zf.read(names[-1]))['files']['assets/private/audio/audio_manifest.json']
            self.assertEqual(meta,{'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})

    def test_missing_installation(self):
        with tempfile.TemporaryDirectory() as temp:
            with self.assertRaises(RuntimeError):export_android.export(Path(temp),Path(temp)/'o.zip')


if __name__=='__main__':
    unittest.main()
