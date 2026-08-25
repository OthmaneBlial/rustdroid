import 'package:flutter_test/flutter_test.dart';

import 'package:rustdroid_flutter_fixture/main.dart';

void main() {
  testWidgets('launches the public fixture message', (WidgetTester tester) async {
    await tester.pumpWidget(const RustDroidFixtureApp());

    expect(find.text('RustDroid Flutter fixture launched'), findsOneWidget);
  });
}
