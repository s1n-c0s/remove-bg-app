import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

function App() {
  const [image, setImage] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Listen for file drops on the window
    const unlisten = getCurrentWindow().listen<{ paths: string[] }>("tauri://drag-drop", (event) => {
      const paths = event.payload.paths;
      if (paths.length > 0) {
        processFilePath(paths[0]);
      }
    });

    return () => {
      unlisten.then(f => f());
    };
  }, []);

  async function processFilePath(path: string) {
    const ext = path.split('.').pop()?.toLowerCase();
    const supported = ['jpg', 'jpeg', 'webp', 'heic', 'png'];
    
    if (ext && supported.includes(ext)) {
      setLoading(true);
      setError(null);
      try {
        const result: string = await invoke("remove_bg", { path });
        setImage(result);
      } catch (err) {
        setError(String(err));
      } finally {
        setLoading(false);
      }
    } else {
      setError(`Unsupported file format: .${ext}. Please use ${supported.join(', ')}.`);
    }
  }

  async function selectImage() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Images',
          extensions: ['jpg', 'jpeg', 'webp', 'heic', 'png']
        }]
      });

      if (selected && typeof selected === 'string') {
        processFilePath(selected);
      }
    } catch (err) {
      setError(String(err));
      setLoading(false);
    }
  }

  return (
    <div className="container">
      <h1>Background Remover</h1>
      <p>Drop an image or select one below</p>
      <p className="formats">Supports .webp, .heic, .jpg, .png</p>

      <div className="actions">
        <button onClick={selectImage} disabled={loading}>
          {loading ? "Processing..." : "Select Image"}
        </button>
      </div>

      {error && <div className="error">{error}</div>}

      <div className="preview-container">
        {image ? (
          <div className="result">
            <img src={image} alt="Processed" />
            <br />
            <a href={image} download="no-bg.png">Download PNG</a>
          </div>
        ) : (
          <div className="placeholder">
            {loading ? "Applying AI Magic..." : "Drop image here"}
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
