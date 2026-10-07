import { createParameterController } from './parameter-controller.mjs';
import { createPreparedParameterDocument } from './prepared-parameter-document.mjs';

export function createHostParameters(React) {
  const { useEffect, useMemo, useState } = React;
  return function useHostParameters(backend) {
    const [state, setState] = useState(null);
    const [document, setDocument] = useState(null);
    const [error, setError] = useState('');
    const reportError = reason => setError(String(reason));
    const controller = useMemo(() => {
      const invoke = async (name, ...args) => {
        const call = backend?.getNativeFunction(name);
        if (!call) throw new Error(`Host command ${name} is unavailable`);
        const reply = await call(...args);
        if (typeof reply === 'string') throw new Error(reply);
        return reply;
      };
      const documents = createPreparedParameterDocument(() => invoke('getPreparedDocument'),
        setDocument, reportError);
      const parameters = createParameterController(invoke, next => {
        setState(next); void documents.acceptGeneration(next.generation);
      }, reportError);
      return { parameters, documents };
    }, [backend]);
    useEffect(() => {
      if (!backend) { reportError('Open this panel in the Dandrum plugin host.'); return; }
      backend.addEventListener('parameterStateChanged', controller.parameters.accept);
      void controller.parameters.refresh();
      return () => {
        backend.removeEventListener?.('parameterStateChanged', controller.parameters.accept);
        controller.parameters.close(); controller.documents.close();
      };
    }, [backend, controller]);
    return { state, document, error, command: controller.parameters.command, reportError };
  };
}
