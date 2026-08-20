import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import type { MobDropGroup } from "../types";
import { Field, Modal } from "./shared";

export function CreateMobModal({
  onClose,
  onCreate,
}: {
  onClose: () => void;
  onCreate: (group: MobDropGroup) => void;
}) {
  const [name, setName] = useState("");
  const [namePreview, setNamePreview] = useState("");
  const [mobVnum, setMobVnum] = useState("");
  const [type, setType] = useState("drop");

  useEffect(() => {
    if (!name.trim()) {
      setNamePreview("");
      return;
    }
    invoke<string>("sanitize_mob_drop_group_name", { name })
      .then(setNamePreview)
      .catch(() => setNamePreview(""));
  }, [name]);

  function submit() {
    const vnum = Number(mobVnum);
    if (!namePreview || !Number.isFinite(vnum) || vnum <= 0) return;
    onCreate({ name: namePreview, mob_vnum: vnum, drop_type: type.trim() || "drop", items: [] });
  }

  return (
    <Modal onClose={onClose}>
      <div className="space-y-3">
        <p className="text-sm font-medium">Neuen Mob-Eintrag anlegen</p>
        <Field label="Name">
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="z.B. Wildhund"
            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </Field>
        {name.trim() && (
          <p className="text-xs text-muted-foreground">
            Wird gespeichert als: <code>{namePreview || "…"}</code>
          </p>
        )}
        <Field label="Mob-VNUM">
          <input
            type="number"
            value={mobVnum}
            onChange={(e) => setMobVnum(e.target.value)}
            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </Field>
        <Field label="Type">
          <input
            value={type}
            onChange={(e) => setType(e.target.value)}
            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </Field>
        <Button onClick={submit} disabled={!namePreview || !mobVnum}>
          Anlegen
        </Button>
      </div>
    </Modal>
  );
}
