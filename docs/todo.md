# TODO

- Make sure display file creation time in the image cards in import window.

# Impeccable

Action Summary                                                                                                                                                 
                                                                                                                                                                
 1. $impeccable harden — the whole cluster, one pass. Scope it to these four things:                                                                            
                                                                                                                                                                
 - Keyboard reachability. Bind Window::focus_next/focus_prev (gpui 0.2.2 exposes them but maps no key — this is why Tab does nothing), add track_focus + a      
   visible focus ring to the six shared helpers (button, chip, primary_button, danger_button, sidebar_nav, timezone_choice), roving-tabindex arrow navigation   
   in the media grid, Enter/Space activation, Escape → back one level on every page (today select_none early-returns unless page == Browser), and bind the      
   seven orphaned actions (MarkSelectedImported, CancelImport, ShowHistory, ReconcileLibrary, CancelReconcile, AdjustClock, DiscoverSources).                   
 - Contrast floors. muted on border at 4.27–4.48:1 on the "Import blocked" pill (6 of 8 scheme/mode combos, render.rs:1008-1010); the text input's only         
   boundary at 1.44–1.56:1 (render.rs:1781-1792); the Selected overlay badge at 1.19:1 over a bright frame in light mode — apply the black scrim DESIGN.md      
   already prescribes for image overlays.                                                                                                                       
 - Target sizes. Thumbnail-size and session-gap steppers at 24×28px, timezone rows at 30.65px, template-segment chips at 32.65px, and the tile                  
   name/status/disclosure hit boxes at 22.65/19.42/19.42px — all under the 36px floor the design system sets for itself.                                        
 - Missing states on interactive controls. danger_button has no hover state at all (the two destructive confirmations are the only controls with zero pointer   
   feedback), and Mount / Refresh devices / Scan for cameras stay clickable while relabelled "Mounting…", so they're re-entrant.                                
                                                                                                                                                                
 2. $impeccable polish — final pass to confirm the fixes hold across all four schemes in both modes and close the critique snapshot.                            
                                                                                                                                                                
 Two honest boundaries before you commit to this pass:                                                                                                          
                                                                                                                                                                
 - Screen-reader support cannot be delivered here. gpui 0.2.2 has no accesskit dependency, so no role, name or state can reach assistive technology regardless  
   of what this pass does. Binding focus traversal and adding focus rings makes the app keyboard-operable; it does not make it screen-reader-operable. That     
   needs a framework-level change, and I won't claim otherwise.                                                                                                 
 - Two adjacent defects sit just outside (a) and I've left them out to respect your scope: the sidebar_nav truncation guard that's missing while button() has   
   it (12 of 18 rail labels overflow at the 184px compact rail, worst by 47.9px), and the missing per-tile error state that renders a failed thumbnail          
   identically to one still loading. Say the word and I'll fold either into this pass.                                                                          
                                                                                                                                                                
 Still open from the critique, and cheapest to decide now rather than after:                                                                                    
                                                                                                                                                                
 - Q2 — Ctrl+I on the Review screen (cluster c). harden rewrites the binding set, so if you want that gate changed it's cheaper to include than to revisit.     
   Your options were (a) make it inert on Review so only the button confirms, (b) keep it but make the second press focus Confirm import, or (c) deliberate,    
   just document it.                                                                                                                                            
 - Q4 — design specificity (no screen treats a photograph as the subject). Independent of this pass. 

