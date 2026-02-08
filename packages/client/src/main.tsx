import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import NeuralUiDemo from './demo/NeuralUiDemo.tsx'
import { shouldUseUiDemo } from './demo/demoMode.ts'

const renderDemo = shouldUseUiDemo(window.location.pathname, window.location.search)

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    {renderDemo ? <NeuralUiDemo /> : <App />}
  </StrictMode>,
)
