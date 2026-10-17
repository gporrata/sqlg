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
          {many ? "Save changes to the open tabs?" : "Save changes to this tab?"}
        </div>
        <p className="confirm-copy">Unsaved editor text is kept in the session only when you quit the app.</p>
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
