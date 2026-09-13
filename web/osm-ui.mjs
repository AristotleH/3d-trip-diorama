import { fetchOSM, convertOSM, validateCenter } from './osm.mjs?v=coverage-1';

export function setupOSM({ onStart, onScene, onError }) {
  const form=document.querySelector('#osm-form'),button=document.querySelector('#osm-build'),cancel=document.querySelector('#osm-cancel'),close=document.querySelector('#osm-close'),details=document.querySelector('#osm-details');
  let controller;
  button.disabled=false;
  async function load({lat,lon,radius}) {
    try{validateCenter(lat,lon,radius);}catch(error){onError(error.message);return;}
    form.elements.latitude.value=String(lat);
    form.elements.longitude.value=String(lon);
    form.elements.radius.value=String(radius);
    controller?.abort();controller=new AbortController();const current=controller;
    const isCurrent=onStart();button.disabled=true;cancel.hidden=false;
    const timer=setTimeout(()=>current.abort('timeout'),35000);
    try {
      const data=await fetchOSM(lat,lon,radius,current.signal);
      if(current.signal.aborted || !isCurrent())return;
      const scene=convertOSM(data,{lat,lon,radius});
      if(isCurrent())onScene(scene);
    } catch(error) {
      if(isCurrent())onError(current.signal.aborted ? (current.signal.reason==='timeout'?'OSM request timed out. Try a smaller radius.':'Import canceled.') : error.message);
    } finally {
      clearTimeout(timer);
      if(controller===current){button.disabled=false;cancel.hidden=true;}
    }
  }
  form.addEventListener('submit', event=>{
    event.preventDefault();
    return load({lat:Number(form.elements.latitude.value),lon:Number(form.elements.longitude.value),radius:Number(form.elements.radius.value)});
  });
  cancel.addEventListener('click',()=>controller?.abort());
  close.addEventListener('click',()=>{ details.open=false; });
  return {load};
}
