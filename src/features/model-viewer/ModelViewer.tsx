import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { Button } from "@/components/ui/button";
import { FolderOpen } from "lucide-react";

interface Gr2Mesh {
  name: string;
  vertices: number[];
  normals: number[];
  uvs: number[];
  indices: number[];
}

interface Gr2ModelInfo {
  name: string;
  bone_count: number;
  meshes: Gr2Mesh[];
  skipped_skinned_meshes: number;
}

interface SceneRefs {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  renderer: THREE.WebGLRenderer;
  controls: OrbitControls;
  meshGroup: THREE.Group;
}

export function ModelViewer() {
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<SceneRefs | null>(null);

  const [dllPath, setDllPath] = useState("");
  const [gr2Path, setGr2Path] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<Gr2ModelInfo | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1e1e1e);

    const camera = new THREE.PerspectiveCamera(
      50,
      container.clientWidth / container.clientHeight,
      0.1,
      10000,
    );
    camera.position.set(100, 100, 200);

    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(container.clientWidth, container.clientHeight);
    container.appendChild(renderer.domElement);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;

    scene.add(new THREE.AmbientLight(0xffffff, 0.6));
    const directional = new THREE.DirectionalLight(0xffffff, 1.2);
    directional.position.set(200, 300, 200);
    scene.add(directional);
    scene.add(new THREE.GridHelper(200, 20, 0x444444, 0x2a2a2a));

    const meshGroup = new THREE.Group();
    scene.add(meshGroup);

    sceneRef.current = { scene, camera, renderer, controls, meshGroup };

    let frameId = 0;
    const animate = () => {
      frameId = requestAnimationFrame(animate);
      controls.update();
      renderer.render(scene, camera);
    };
    animate();

    const handleResize = () => {
      camera.aspect = container.clientWidth / container.clientHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(container.clientWidth, container.clientHeight);
    };
    window.addEventListener("resize", handleResize);

    return () => {
      cancelAnimationFrame(frameId);
      window.removeEventListener("resize", handleResize);
      controls.dispose();
      renderer.dispose();
      container.removeChild(renderer.domElement);
    };
  }, []);

  function renderModel(model: Gr2ModelInfo) {
    const refs = sceneRef.current;
    if (!refs) return;
    const { camera, controls, meshGroup } = refs;

    for (const child of [...meshGroup.children]) {
      meshGroup.remove(child);
      if (child instanceof THREE.Mesh) {
        child.geometry.dispose();
        (child.material as THREE.Material).dispose();
      }
    }

    for (const mesh of model.meshes) {
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute(
        "position",
        new THREE.Float32BufferAttribute(mesh.vertices, 3),
      );
      geometry.setAttribute(
        "normal",
        new THREE.Float32BufferAttribute(mesh.normals, 3),
      );
      geometry.setAttribute("uv", new THREE.Float32BufferAttribute(mesh.uvs, 2));
      geometry.setIndex(new THREE.Uint32BufferAttribute(mesh.indices, 1));

      const material = new THREE.MeshPhongMaterial({
        color: 0xcfcfcf,
        side: THREE.DoubleSide,
      });
      meshGroup.add(new THREE.Mesh(geometry, material));
    }

    const box = new THREE.Box3().setFromObject(meshGroup);
    const center = box.getCenter(new THREE.Vector3());
    const size = Math.max(box.getSize(new THREE.Vector3()).length(), 1);
    meshGroup.position.sub(center);
    camera.position.set(size * 0.5, size * 0.5, size * 0.8);
    camera.near = size / 100;
    camera.far = size * 100;
    camera.updateProjectionMatrix();
    controls.target.set(0, 0, 0);
    controls.update();
  }

  async function pickDllPath() {
    const selected = await open({
      multiple: false,
      title: "granny2.dll auswählen",
      filters: [{ name: "Granny2 Library", extensions: ["dll"] }],
    });
    if (typeof selected === "string") {
      setDllPath(selected);
    }
  }

  async function pickGr2Path() {
    const selected = await open({
      multiple: false,
      title: ".gr2-Modelldatei auswählen",
      filters: [{ name: "Granny3D Model", extensions: ["gr2"] }],
    });
    if (typeof selected === "string") {
      setGr2Path(selected);
    }
  }

  async function handleLoad() {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<Gr2ModelInfo>("load_gr2_model", {
        grannyDllPath: dllPath,
        gr2Path,
      });
      setInfo(result);
      renderModel(result);
    } catch (e) {
      setError(String(e));
      setInfo(null);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex h-full flex-col gap-4">
      <h1 className="text-2xl font-semibold">{t("nav.modelViewer")}</h1>

      <div className="space-y-2 rounded-lg border border-border bg-card p-3">
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={pickDllPath} className="shrink-0">
            <FolderOpen className="size-4" />
            granny2.dll auswählen
          </Button>
          <span className="truncate text-sm text-muted-foreground">
            {dllPath || "Keine Datei ausgewählt"}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={pickGr2Path} className="shrink-0">
            <FolderOpen className="size-4" />
            .gr2-Datei auswählen
          </Button>
          <span className="truncate text-sm text-muted-foreground">
            {gr2Path || "Keine Datei ausgewählt"}
          </span>
        </div>
        <p className="text-xs text-muted-foreground">
          Tipp: <code>granny2.dll</code> liegt normalerweise direkt im
          Hauptordner deines Metin2-Clients (neben der .exe). .gr2-Modelle
          findest du im <code>pack</code>-Ordner des Clients, meist unter{" "}
          <code>ymir work\npc\...</code>, <code>ymir work\monster\...</code>{" "}
          oder <code>ymir work\item\...</code>. Falls diese Ordner nicht
          existieren, liegen die Assets noch gepackt (.epk) vor und müssen
          zuerst entpackt werden.
        </p>
        <Button onClick={handleLoad} disabled={loading || !dllPath || !gr2Path}>
          {loading ? "Lädt…" : "Laden"}
        </Button>
      </div>

      {error && <p className="text-sm text-destructive">{error}</p>}
      {info && (
        <p className="text-sm text-muted-foreground">
          {info.name} — {info.bone_count} Bones, {info.meshes.length} Mesh(es)
          {info.skipped_skinned_meshes > 0 &&
            ` (${info.skipped_skinned_meshes} skinned, noch nicht unterstützt)`}
        </p>
      )}

      <div
        ref={containerRef}
        className="min-h-0 flex-1 overflow-hidden rounded-lg border border-border"
      />
    </div>
  );
}
