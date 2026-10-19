import { useApp } from "../app/store";

export function ClosePrompt() {
  const prompt = useApp((state) => state.closePrompt);
  const answer = useApp((state) => state.answerClose);
  if (!prompt) return null;
  const many = prompt.tabIds.length > 1;
  return (
    <div className="mask" role="presentation">
      <div className="modal confirm" role="dialog" aria-labelledby="close-title">
        <div className="mt" id="close-title">
          {many ? "Save changes to the open files?" : "Save changes to this file?"}
        </div>
        <p className="confirm-copy">This editor was loaded from a file and has been modified.</p>
        <div className="mf">
          <button className="btn" type="button" onClick={() => answer("cancel")}>
            Cancel
          </button>
          <button className="btn" type="button" onClick={() => answer("discard")}>
            Don&apos;t Save
          </button>
          <button className="btn p" type="button" onClick={() => answer("save")}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
