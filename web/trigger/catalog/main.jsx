import React from 'react';
import {createRoot} from 'react-dom/client';
import {CatalogController} from './CatalogController.jsx';
import '../components/design-system/styles.css';
import '../components/source-hover.css';
import '../styles.css';
import './styles.css';
const root=createRoot(document.getElementById('root'));root.render(<CatalogController/>);if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());
