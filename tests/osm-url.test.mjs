import { test } from 'node:test';
import assert from 'node:assert/strict';
import { osmHash, parseOSMHash } from '../web/osm-url.mjs';

test('coordinates round-trip without losing precision, including zero',()=>{
 for(const params of [{lat:37.774530123,lon:-122.389813456,radius:1000},{lat:0,lon:0,radius:100}])
  assert.deepEqual(parseOSMHash(osmHash(params)),params);
 assert.equal(parseOSMHash('#osm'),null);
 assert.equal(parseOSMHash('#glacier-organ'),null);
});
test('missing, duplicate, empty and out-of-range URL inputs are rejected',()=>{
 for(const query of ['lat=0&lon=0','lat=&lon=0&radius=500','lat=0&lat=1&lon=0&radius=500','lat=NaN&lon=0&radius=500','lat=0&lon=0&radius=9999'])
  assert.throws(()=>parseOSMHash(`#osm?${query}`));
});
