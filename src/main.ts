import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { startTheme } from './lib/themeBoot';

startTheme();

const app = mount(App, { target: document.getElementById('app')! });

export default app;
