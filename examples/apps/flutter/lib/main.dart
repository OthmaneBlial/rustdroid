import 'package:flutter/material.dart';

void main() => runApp(const RustDroidFixtureApp());

class RustDroidFixtureApp extends StatelessWidget {
  const RustDroidFixtureApp({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      home: Scaffold(
        body: Center(
          child: Text('RustDroid Flutter fixture launched'),
        ),
      ),
    );
  }
}
