// node tools/convert-osm.mjs overpass.json latitude longitude radius > scene.json
import { readFileSync } from 'node:fs';
import { convertOSM } from '../web/osm.mjs';
try {
  const [file,lat,lon,radius]=process.argv.slice(2);
  if(!file||radius===undefined)throw new Error('Usage: node tools/convert-osm.mjs input.json latitude longitude radius-metres');
  console.log(JSON.stringify(convertOSM(JSON.parse(readFileSync(file,'utf8')),{lat:Number(lat),lon:Number(lon),radius:Number(radius)}),null,2));
}catch(error){console.error(error.message);process.exitCode=1;}
