/* LaRuche App SDK v1
 *
 * This file runs inside an untrusted, opaque-origin iframe. It never receives
 * cookies or a generic HTTP client. The host transfers one private MessagePort
 * to the exact iframe instance after load; every capability call uses that
 * revocable channel.
 */
(function(global){
  'use strict';
  if(global.LaRucheApp) return;

  var port=null;
  var context=null;
  var sequence=0;
  var pending=new Map();
  var readyResolve, readyReject;
  var readyPromise=new Promise(function(resolve,reject){ readyResolve=resolve; readyReject=reject; });
  var initTimer=setTimeout(function(){
    if(!port) readyReject(new Error('LaRuche app bridge unavailable'));
  },5000);

  function bridgeError(input){
    var error=new Error((input&&input.message)||'LaRuche app bridge error');
    error.code=(input&&input.code)||'internal_error';
    error.retryable=!!(input&&input.retryable);
    return error;
  }

  function send(message){
    if(!port) throw bridgeError({code:'service_unavailable',message:'Bridge is not connected'});
    port.postMessage(message);
  }

  function onPortMessage(event){
    var message=event.data;
    if(!message || message.v!==1 || typeof message.kind!=='string') return;
    if(message.kind==='host.welcome'){
      context=Object.freeze(message.context||{});
      send({v:1,kind:'app.ready',sessionId:context.sessionId});
      readyResolve(context);
      return;
    }
    if(message.kind!=='response' || typeof message.id!=='string') return;
    var request=pending.get(message.id);
    if(!request) return;
    pending.delete(message.id);
    clearTimeout(request.timer);
    if(message.ok) request.resolve(message.result);
    else request.reject(bridgeError(message.error));
  }

  function onInit(event){
    var message=event.data;
    if(port || event.source!==global.parent || !message || message.kind!=='laruche.host.init' || message.v!==1 || event.ports.length!==1) return;
    if(typeof message.nonce!=='string' || message.nonce.length<32) return;
    port=event.ports[0];
    clearTimeout(initTimer);
    global.removeEventListener('message',onInit);
    port.onmessage=onPortMessage;
    port.onmessageerror=function(){ readyReject(bridgeError({code:'invalid_request',message:'Invalid bridge message'})); };
    port.start();
    send({
      v:1,
      kind:'app.hello',
      nonce:message.nonce,
      appId:message.appId,
      viewId:message.viewId,
      apiVersion:1,
      sdkVersion:'1.0.0'
    });
  }

  function call(method,params){
    return readyPromise.then(function(){
      var id='sdk-'+Date.now().toString(36)+'-'+(++sequence).toString(36);
      return new Promise(function(resolve,reject){
        var timer=setTimeout(function(){
          pending.delete(id);
          reject(bridgeError({code:'timeout',message:'Host request timed out',retryable:true}));
        },10000);
        pending.set(id,{resolve:resolve,reject:reject,timer:timer});
        try{ send({v:1,kind:'request',id:id,method:method,params:params||{}}); }
        catch(error){ clearTimeout(timer); pending.delete(id); reject(error); }
      });
    });
  }

  global.addEventListener('message',onInit);
  var api=Object.freeze({
    version:'1.0.0',
    ready:function(){ return readyPromise; },
    call:call,
    storage:Object.freeze({
      get:function(key){ return call('storage.get',{key:key}).then(function(result){ return result.value; }); },
      set:function(key,value){ return call('storage.set',{key:key,value:value}); },
      delete:function(key){ return call('storage.delete',{key:key}); },
      list:function(prefix){ return call('storage.list',{prefix:prefix||''}).then(function(result){ return result.keys; }); }
    }),
    ui:Object.freeze({
      setTitle:function(title){ return call('ui.setTitle',{title:title}); },
      setDirty:function(dirty){ return call('ui.setDirty',{dirty:!!dirty}); },
      requestDetach:function(){ return call('ui.requestDetach',{}); },
      close:function(){ return call('ui.close',{}); }
    })
  });
  global.LaRucheApp=api;
  // Compatibility for packages authored before the product name became Apps.
  global.LaRucheAddon=api;
})(window);
