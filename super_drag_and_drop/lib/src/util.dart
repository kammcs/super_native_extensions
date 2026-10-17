import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';

class SimpleNotifier extends ChangeNotifier {
  void notify() {
    super.notifyListeners();
  }
}

/// Hit tests [position] in the Flutter view with [viewId]. Falls back to the
/// implicit view, or to the first view with a size when there is no implicit
/// view (i.e. when all views are created through the windowing API).
void hitTestInView(HitTestResult result, Offset position, {int? viewId}) {
  final dispatcher = GestureBinding.instance.platformDispatcher;
  FlutterView? view = viewId != null ? dispatcher.view(id: viewId) : null;
  view ??= dispatcher.implicitView;
  if (view == null) {
    for (final candidate in dispatcher.views) {
      view ??= candidate;
      if (!candidate.physicalSize.isEmpty) {
        view = candidate;
        break;
      }
    }
  }
  if (view != null) {
    GestureBinding.instance.hitTestInView(result, position, view.viewId);
  }
}
