(async function(){
  'use strict';
  const status=document.getElementById('status');
  try{
    await LaRucheApp.ready();
    const response=await fetch('./add.wasm',{credentials:'omit'});
    if(!response.ok) throw new Error('Module indisponible : '+response.status);
    const {instance}=await WebAssembly.instantiateStreaming(response,{});
    const calculate=function(event){
      if(event) event.preventDefault();
      const a=document.getElementById('a').valueAsNumber;
      const b=document.getElementById('b').valueAsNumber;
      if(!Number.isInteger(a)||!Number.isInteger(b)) return;
      document.getElementById('result').textContent=String(instance.exports.add(a,b));
    };
    document.getElementById('calculator').addEventListener('submit',calculate);
    document.getElementById('calculate').disabled=false;
    status.textContent='WebAssembly actif';
    calculate();
  }catch(error){status.textContent='Erreur : '+error.message;}
})();
